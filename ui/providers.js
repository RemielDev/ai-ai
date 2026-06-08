// Provider catalog - single source of truth for the UI side.
// The Rust side has its own copy in src-tauri/src/settings.rs.

window.PROVIDERS = {
  gemini: {
    label: "Google Gemini",
    keyPrefix: "AIza",
    consoleUrl: "https://aistudio.google.com/apikey",
    defaultModel: "gemini-2.0-flash",
    cost: "≈ $0.0001 / call",
    modelHint: "Try gemini-2.0-flash (fast + cheap) or gemini-1.5-pro (quality).",
  },
  openai: {
    label: "OpenAI",
    keyPrefix: "sk-",
    consoleUrl: "https://platform.openai.com/api-keys",
    defaultModel: "gpt-4o-mini",
    cost: "≈ $0.0003 / call",
    modelHint: "Try gpt-4o-mini (fast + cheap) or gpt-4o (quality).",
  },
  openrouter: {
    label: "OpenRouter",
    keyPrefix: "sk-or-",
    consoleUrl: "https://openrouter.ai/keys",
    defaultModel: "google/gemini-2.0-flash-exp:free",
    cost: "free tier available",
    modelHint: "Pick any model slug from openrouter.ai/models. The :free suffix gives free quota.",
  },
  anthropic: {
    label: "Anthropic (Claude)",
    keyPrefix: "sk-ant-",
    consoleUrl: "https://console.anthropic.com/settings/keys",
    defaultModel: "claude-haiku-4-6",
    cost: "≈ $0.001 / call",
    modelHint: "Try claude-haiku-4-6 (fast) or claude-sonnet-4-7 (best quality).",
  },
};
