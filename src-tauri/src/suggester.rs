//! Suggester — multi-provider AI client.
//!
//! Providers: Anthropic, OpenAI, OpenRouter, Google Gemini.
//! Each provider has its own endpoint shape; we normalize all of them
//! to "give me plain text" then parse suggestions JSON at the top level.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::reader::ChatSnapshot;
use crate::settings::{Provider, Settings, StylePreset};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Suggestion {
    pub text: String,
}

pub struct Suggester {
    provider: Provider,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl Suggester {
    pub fn new(provider: Provider, api_key: String, model: String) -> Self {
        Self {
            provider,
            api_key,
            model,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(25))
                .build()
                .expect("build reqwest client"),
        }
    }

    /// Cheap ping to verify the key works for this provider.
    pub async fn validate(&self) -> Result<()> {
        let _ = self
            .call("You answer with one word.", "Reply with: ok", 8)
            .await?;
        Ok(())
    }

    pub async fn generate_followups(
        &self,
        snapshot: &ChatSnapshot,
        settings: &Settings,
        seed: u32,
        prior: &[Suggestion],
    ) -> Result<Vec<Suggestion>> {
        if snapshot.last_assistant_msg.trim().is_empty() {
            anyhow::bail!("no recent assistant message to react to");
        }

        let n = settings.suggestion_count.max(3).min(6);
        let style = settings.style_preset.directive();

        let steer_block = if !snapshot.chat_input_text.trim().is_empty() {
            format!(
                "\n\nThe user has already started typing their next prompt: \"{}\". \
Bias your suggestions toward extending, refining, or sharpening that direction.",
                snapshot.chat_input_text.replace('"', "'")
            )
        } else {
            String::new()
        };

        let variation_block = if seed > 0 && !prior.is_empty() {
            let listed: Vec<String> = prior.iter().map(|s| format!("- {}", s.text)).collect();
            format!(
                "\n\nYou have already suggested these before — generate {} that explore DIFFERENT angles and phrasings. Avoid restating these:\n{}",
                n,
                listed.join("\n")
            )
        } else {
            String::new()
        };

        let style_block = if style.is_empty() {
            String::new()
        } else {
            format!("\n\nStyle: {}", style)
        };

        let system_prompt = format!(
            "You are a prompt copilot helping a user decide what to ask the AI next. \
Given the most recent AI response, generate {n} short follow-up prompts the user could send. \
Each suggestion: one sentence, 8–20 words, written exactly as the user would type it (first person, casual). \
Vary the angles: deeper, challenge an assumption, request an example, ask for a contrasting view, \
ask for the next concrete step.\n\n\
Return ONLY a JSON object: {{\"suggestions\": [\"...\", \"...\"]}}. \
No prose, no markdown fence.{steer_block}{variation_block}{style_block}"
        );

        let user_content = format!(
            "Most recent AI response:\n---\n{}\n---",
            truncate(&snapshot.last_assistant_msg, 6000)
        );

        let text_out = self.call(&system_prompt, &user_content, 700).await?;
        let cleaned = strip_code_fence(&text_out);
        let json_val: Value = serde_json::from_str(&cleaned)
            .with_context(|| format!("decode suggestions JSON from: {}", truncate(&cleaned, 200)))?;
        let arr = json_val
            .get("suggestions")
            .and_then(|v| v.as_array())
            .context("missing 'suggestions' array")?;

        let suggestions: Vec<Suggestion> = arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| Suggestion { text: s.trim().to_string() }))
            .filter(|s| !s.text.is_empty())
            .take(n as usize)
            .collect();

        if suggestions.is_empty() {
            anyhow::bail!("zero usable suggestions in response");
        }
        Ok(suggestions)
    }

    pub async fn improve_prompt(&self, draft: &str) -> Result<String> {
        let system_prompt = "Rewrite the following draft prompt to be clearer, more specific, \
and more likely to get a useful response from an AI assistant. Preserve the user's intent and tone. \
Do not add unrelated requirements or new questions. Return ONLY the rewritten prompt as plain text, \
no preamble, no quotes, no markdown fence.";
        let user_content = format!("DRAFT:\n{}", truncate(draft, 4000));
        let text_out = self.call(system_prompt, &user_content, 700).await?;
        let cleaned = strip_code_fence(&text_out).trim().to_string();
        if cleaned.is_empty() {
            anyhow::bail!("provider returned an empty improvement");
        }
        Ok(cleaned)
    }

    // ---------------- Provider dispatch ----------------

    async fn call(&self, system: &str, user: &str, max_tokens: u32) -> Result<String> {
        match self.provider {
            Provider::Anthropic => self.call_anthropic(system, user, max_tokens).await,
            Provider::OpenAI => {
                self.call_openai(
                    "https://api.openai.com/v1/chat/completions",
                    system,
                    user,
                    max_tokens,
                    None,
                )
                .await
            }
            Provider::OpenRouter => {
                self.call_openai(
                    "https://openrouter.ai/api/v1/chat/completions",
                    system,
                    user,
                    max_tokens,
                    Some(("HTTP-Referer", "https://aiai.app")),
                )
                .await
            }
            Provider::Gemini => self.call_gemini(system, user, max_tokens).await,
        }
    }

    async fn call_anthropic(&self, system: &str, user: &str, max_tokens: u32) -> Result<String> {
        let body = json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "system": system,
            "messages": [{"role": "user", "content": user}],
        });
        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("POST anthropic")?;
        let status = resp.status();
        let raw = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("Anthropic {}: {}", status, truncate(&raw, 400));
        }
        let v: Value = serde_json::from_str(&raw).context("decode anthropic json")?;
        let text = v["content"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|b| {
                        if b.get("type").and_then(|t| t.as_str()) == Some("text") {
                            b.get("text").and_then(|t| t.as_str()).map(String::from)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();
        if text.is_empty() {
            anyhow::bail!("Anthropic returned no text content");
        }
        Ok(text)
    }

    async fn call_openai(
        &self,
        url: &str,
        system: &str,
        user: &str,
        max_tokens: u32,
        extra_header: Option<(&str, &str)>,
    ) -> Result<String> {
        let body = json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user",   "content": user},
            ],
        });
        let mut req = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .header("content-type", "application/json")
            .json(&body);
        if let Some((k, v)) = extra_header {
            req = req.header(k, v);
        }
        let resp = req.send().await.context("POST openai-shape")?;
        let status = resp.status();
        let raw = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("OpenAI-shape {}: {}", status, truncate(&raw, 400));
        }
        let v: Value = serde_json::from_str(&raw).context("decode openai json")?;
        let text = v["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        if text.is_empty() {
            anyhow::bail!("OpenAI-shape returned no text content");
        }
        Ok(text)
    }

    async fn call_gemini(&self, system: &str, user: &str, max_tokens: u32) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );
        let body = json!({
            "systemInstruction": {"parts": [{"text": system}]},
            "contents": [{"role": "user", "parts": [{"text": user}]}],
            "generationConfig": {
                "maxOutputTokens": max_tokens,
                "temperature": 0.8,
            }
        });
        let resp = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("POST gemini")?;
        let status = resp.status();
        let raw = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("Gemini {}: {}", status, truncate(&raw, 400));
        }
        let v: Value = serde_json::from_str(&raw).context("decode gemini json")?;
        let text = v["candidates"][0]["content"]["parts"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| p.get("text").and_then(|t| t.as_str()).map(String::from))
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();
        if text.is_empty() {
            anyhow::bail!("Gemini returned no text content");
        }
        Ok(text)
    }
}

pub fn current_suggester(provider: Provider, api_key: String, settings: &Settings) -> Suggester {
    let model = if settings.model.is_empty() {
        default_model(provider).to_string()
    } else {
        settings.model.clone()
    };
    Suggester::new(provider, api_key, model)
}

pub fn default_model(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic => "claude-haiku-4-6",
        Provider::OpenAI => "gpt-4o-mini",
        Provider::OpenRouter => "google/gemini-2.0-flash-exp:free",
        Provider::Gemini => "gemini-2.0-flash",
    }
}

#[allow(dead_code)]
pub fn style_directive(p: StylePreset) -> &'static str {
    p.directive()
}

fn strip_code_fence(s: &str) -> String {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("```json") {
        return rest
            .trim_start_matches('\n')
            .trim_end_matches("```")
            .trim_end_matches('\n')
            .to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("```") {
        return rest
            .trim_start_matches('\n')
            .trim_end_matches("```")
            .trim_end_matches('\n')
            .to_string();
    }
    // Some models return prose before/after the JSON. Try to find the first {...}.
    if !trimmed.starts_with('{') {
        if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
            if end > start {
                return trimmed[start..=end].to_string();
            }
        }
    }
    trimmed.to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut end = max;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}
