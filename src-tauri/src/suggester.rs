//! Suggester — calls Anthropic API with the user's BYO key.
//!
//! Two methods:
//!   * `generate_followups` — N suggestion strings from a chat snapshot.
//!   * `improve_prompt`     — rewrite a single draft for clarity.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::reader::ChatSnapshot;
use crate::settings::{Settings, StylePreset};

const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Suggestion {
    pub text: String,
}

pub struct Suggester {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl Suggester {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .expect("build reqwest client"),
        }
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
Bias your suggestions toward extending, refining, or sharpening that direction. \
If their draft is a partial thought, suggestions should complete it in different ways.",
                snapshot.chat_input_text.replace('"', "'")
            )
        } else {
            String::new()
        };

        let variation_block = if seed > 0 && !prior.is_empty() {
            let listed: Vec<String> = prior
                .iter()
                .map(|s| format!("- {}", s.text))
                .collect();
            format!(
                "\n\nYou have already suggested these before — generate {} that explore DIFFERENT angles, phrasings, and depths. Avoid restating these:\n{}",
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
            "You are a prompt copilot helping a user decide what to ask Claude next. \
Given the most recent Claude response, generate {n} short follow-up prompts the user could send. \
Each suggestion: one sentence, 8–20 words, written exactly as the user would type it (first person, casual). \
Vary the angles: go deeper, challenge an assumption, request an example, request a contrasting view, \
ask for the next concrete step, ask for a summary, etc.\n\n\
Return ONLY a JSON object: {{\"suggestions\": [\"...\", \"...\"]}}. \
No prose, no markdown fence.{steer_block}{variation_block}{style_block}"
        );

        let user_content = format!(
            "Claude's most recent response:\n---\n{}\n---",
            truncate(&snapshot.last_assistant_msg, 6000)
        );

        let body = json!({
            "model": self.model,
            "max_tokens": 600,
            "system": system_prompt,
            "messages": [{"role": "user", "content": user_content}],
        });

        let resp = self
            .client
            .post(ANTHROPIC_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("send to Anthropic")?;

        let status = resp.status();
        let raw = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("Anthropic returned {}: {}", status, truncate(&raw, 400));
        }

        let parsed: AnthropicMessage = serde_json::from_str(&raw)
            .with_context(|| format!("decode Anthropic response: {}", truncate(&raw, 200)))?;
        let text_out = parsed
            .content
            .into_iter()
            .filter_map(|b| if b.kind == "text" { Some(b.text) } else { None })
            .collect::<Vec<_>>()
            .join("");

        let cleaned = strip_code_fence(&text_out);
        let json_val: serde_json::Value = serde_json::from_str(&cleaned)
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
            anyhow::bail!("Anthropic returned zero usable suggestions");
        }
        Ok(suggestions)
    }

    pub async fn improve_prompt(&self, draft: &str) -> Result<String> {
        let system_prompt = "Rewrite the following draft prompt to be clearer, more specific, \
and more likely to get a useful response from an AI assistant. Preserve the user's intent and tone. \
Do not add unrelated requirements or new questions. Return ONLY the rewritten prompt as plain text, \
no preamble, no quotes, no markdown fence.";

        let body = json!({
            "model": self.model,
            "max_tokens": 600,
            "system": system_prompt,
            "messages": [{
                "role": "user",
                "content": format!("DRAFT:\n{}", truncate(draft, 4000))
            }],
        });

        let resp = self
            .client
            .post(ANTHROPIC_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("send to Anthropic")?;

        let status = resp.status();
        let raw = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("Anthropic returned {}: {}", status, truncate(&raw, 400));
        }
        let parsed: AnthropicMessage = serde_json::from_str(&raw)
            .with_context(|| format!("decode Anthropic response: {}", truncate(&raw, 200)))?;
        let text_out = parsed
            .content
            .into_iter()
            .filter_map(|b| if b.kind == "text" { Some(b.text) } else { None })
            .collect::<Vec<_>>()
            .join("");
        let cleaned = strip_code_fence(&text_out).trim().to_string();
        if cleaned.is_empty() {
            anyhow::bail!("Anthropic returned an empty improvement");
        }
        Ok(cleaned)
    }
}

pub fn current_suggester(api_key: String, settings: &Settings) -> Suggester {
    let model = settings.model.clone();
    let model = if model.is_empty() {
        "claude-haiku-4-6".into()
    } else {
        model
    };
    Suggester::new(api_key, model)
}

#[allow(dead_code)]
pub fn style_directive(p: StylePreset) -> &'static str {
    p.directive()
}

#[derive(Debug, Deserialize)]
struct AnthropicMessage {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
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
