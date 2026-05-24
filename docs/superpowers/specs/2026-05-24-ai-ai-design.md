# AI-AI — Design Spec

**Status:** Approved for implementation planning
**Date:** 2026-05-24
**Owner:** Remie
**Platform (v1):** Windows 11, Claude Desktop app (current version)

---

## 1. Product summary

AI-AI is a Windows tray app that gives Claude Desktop users an AI-powered copilot for what to ask next, and a one-keystroke prompt improver. It runs locally, uses the user's own Anthropic API key (BYOK), and sells as a one-time-license indie tool.

**Two features behind one tool:**

1. **Suggest next prompt** (`Ctrl+Shift+Space`) — reads the most recent assistant response from the Claude Desktop window, generates 3–6 follow-up prompts, and shows them in a floating always-on-top overlay anchored above Claude's chat input.
2. **Improve current prompt** (`Ctrl+Shift+Enter` while Claude's chat input is focused) — takes whatever the user has typed and rewrites it as a sharper, more effective prompt, replacing the contents in place.

**Context-aware hotkey overload — `Ctrl+Shift+Enter`:**

| Focused window | Action |
|---|---|
| AI-AI overlay | Paste highlighted suggestion into Claude's chat input (do not auto-send by default) |
| Claude Desktop chat input | Improve the current draft in place |
| Anything else | No-op |

---

## 2. Architecture

### Stack
- **Language / runtime:** Rust (core) + Tauri (overlay UI via embedded webview)
- **Why Tauri over Electron/WPF:** ~3 MB installer, native Rust access to Windows UIA, modern web-based overlay (animations, theming), clean license-gating, cross-platform path open for a future macOS v2
- **OS APIs:** Windows UI Automation (`uiautomation` crate), `RegisterHotKey` via `windows` crate, `SendInput` for fallback paste
- **HTTP:** `reqwest` for Anthropic API calls
- **Secrets:** Windows Credential Manager (`wincredentials` crate) for API key storage
- **Installer:** NSIS or WiX via `tauri-bundler`

### Units (each does one thing, communicates through a defined interface)

```
┌─────────────────────────────────────────────────────────┐
│ AI-AI Tray App (Rust + Tauri)                           │
│                                                         │
│  ┌──────────────┐   ┌──────────────┐   ┌─────────────┐ │
│  │ HotkeyDaemon │ → │ ClaudeReader │ → │  Suggester  │ │
│  │ (global keys)│   │ (UIA scrape) │   │ (Anthropic) │ │
│  └──────────────┘   └──────────────┘   └─────────────┘ │
│         ↓                                      ↓        │
│  ┌──────────────┐                      ┌─────────────┐ │
│  │   Injector   │ ← ──── selected ──── │ Overlay UI  │ │
│  │ (paste/send) │                      │ (Tauri web) │ │
│  └──────────────┘                      └─────────────┘ │
└─────────────────────────────────────────────────────────┘
```

| Unit | Responsibility | Public interface | Depends on |
|---|---|---|---|
| **HotkeyDaemon** | Register `Ctrl+Shift+Space` (summon) and `Ctrl+Shift+Enter` (context paste/improve). On fire, inspect focused window and dispatch. | `on_summon()`, `on_context_action()` | `windows::Win32::UI::Input::KeyboardAndMouse` |
| **ClaudeReader** | Locate Claude Desktop's window. Walk UIA tree. Return a `ChatSnapshot { last_assistant_msg: String, last_user_msg: Option<String>, chat_input_text: String, chat_input_bounds: Rect }`. | `snapshot() -> Result<ChatSnapshot>`, `is_claude_focused() -> bool`, `is_chat_input_focused() -> bool` | `uiautomation` crate |
| **Suggester** | Two methods. Owns the variation-seed cache. | `generate_followups(snapshot, opts) -> Vec<Suggestion>`, `improve_prompt(draft) -> String` | `reqwest`, user's API key |
| **Overlay UI** | Tauri webview window, frameless, always-on-top, anchored from `chat_input_bounds`. Renders 3–6 suggestion cards. Arrow-key + mouse nav. Loading & error states. Emits selected suggestion to backend. | Tauri events: `show(suggestions, anchor_rect)`, `hide()`, `selected(text)` | Tauri |
| **Injector** | Write text into Claude's chat input. Primary path: UIA `ValuePattern.SetValue`. Fallback: clipboard set + `SendInput Ctrl+V`. Optionally simulate Enter when `auto_send=true`. | `paste(text, auto_send: bool) -> Result<()>` | UIA + `SendInput` |

**Design rule:** any unit can be unit-tested by mocking its single dependency. ClaudeReader is the only unit with brittle external coupling — it gets the most test coverage and a clear fallback path.

---

## 3. Core flows

### Flow A — Summon suggestions (`Ctrl+Shift+Space`)

1. `HotkeyDaemon` fires. Checks `ClaudeReader.is_claude_focused()`. If false, show toast "Open Claude Desktop first" and abort.
2. `ClaudeReader.snapshot()` returns `ChatSnapshot`.
3. `Suggester.generate_followups(snapshot, opts)` called. Uses `chat_input_text` as steer context if non-empty.
4. `Overlay UI` shown at `chat_input_bounds.top - overlay_height - 8px`, centered horizontally on the chat input. Renders loading skeleton.
5. When suggestions arrive, render 3–6 cards. First card auto-highlighted. Arrow keys navigate. `Esc` dismisses.
6. `Ctrl+Shift+Space` pressed again while overlay open → regenerate with `variation_seed` incremented (see below).

### Flow B — Paste suggestion (`Ctrl+Shift+Enter` while overlay focused)

1. `HotkeyDaemon` fires. Sees overlay window has focus → routes to `Injector`.
2. Frontend emits highlighted card text.
3. `Injector.paste(text, settings.auto_send)`.
4. Overlay closes. If `auto_send=false` (default), focus returns to Claude's chat box.

### Flow C — Improve current prompt (`Ctrl+Shift+Enter` while Claude chat input focused)

1. `HotkeyDaemon` fires. Sees Claude chat input focused → routes to improve path.
2. `ClaudeReader.snapshot()` → grabs `chat_input_text`. If empty, no-op + subtle toast "Type something to improve".
3. Show small "improving…" indicator (1.5 s timeout for visible feedback) near caret position.
4. `Suggester.improve_prompt(draft)` returns improved string.
5. `Injector.paste(improved, auto_send=false)` — replaces chat input contents. Focus stays in chat box.

### Variation-seed logic

`Suggester` keeps an LRU cache (size 8) of `(last_assistant_msg_hash, chat_input_text_hash) -> { seed: u32, prior_suggestions: Vec<String> }`. On `generate_followups`:

- If key not in cache: `seed = 0`, no prior suggestions to exclude.
- If key found: `seed += 1`, append a directive to the system prompt: *"Generate suggestions different in angle and phrasing from these prior suggestions: <list>. Explore unexplored directions."*

This makes repeated hotkey presses with unchanged context produce genuinely new suggestions instead of restating the same ideas.

---

## 4. Suggester prompts (v1 drafts)

### `generate_followups`

System prompt scaffold:
```
You are a prompt copilot. The user is in a conversation with Claude.
Given the most recent assistant message, suggest {n} short follow-up
prompts the user could send next.

Each suggestion: one sentence, 8–20 words, written as the user would type
it. Vary the angles: dig deeper, challenge an assumption, request an
example, ask for a contrasting view, request a concrete next step, etc.

{steer_block}    # only if chat_input_text non-empty
{variation_block}  # only if seed > 0

Return JSON: { "suggestions": ["...", "...", ...] }
```

- `steer_block` example: *"The user is leaning toward: '{chat_input_text}'. Bias suggestions in that direction."*
- `variation_block` example: *"Previously suggested: [...]. Avoid these angles."*

### `improve_prompt`

System prompt scaffold:
```
Rewrite the following draft prompt to be clearer, more specific, and
more likely to get a useful response from an AI assistant. Preserve the
user's intent and tone. Do not add unrelated requirements. Return only
the rewritten prompt, no preamble.

DRAFT:
{draft}
```

Both prompts use `claude-haiku-4-6` by default (fast, cheap). Settings allow switching to Sonnet 4.7 for quality.

---

## 5. Settings, licensing, errors

### Settings panel (tray icon → Settings)

| Setting | Default | Notes |
|---|---|---|
| Anthropic API key | empty | Stored in Windows Credential Manager. Never written to disk plain. Validate on save via a no-op API call. |
| Model | Haiku 4.6 | Dropdown: Haiku (fast/cheap) / Sonnet (quality). |
| Summon hotkey | `Ctrl+Shift+Space` | Rebind with live conflict detection (re-registers on save). |
| Paste/Improve hotkey | `Ctrl+Shift+Enter` | Same. |
| Number of suggestions | 4 | Range 3–6. |
| Suggestion style preset | Default | Default / Concise / Exploratory / Technical — each is a small system-prompt addendum. |
| Auto-send after paste | off | Toggle. When on, `Injector` simulates Enter after paste. |
| Telemetry opt-in | off | Anonymous counts only. See below. |

### Licensing

- Purchase via **Lemon Squeezy** (preferred over Gumroad for international tax handling).
- On purchase, customer receives a license key by email.
- First launch shows trial banner. Trial is 7 days, all features unlocked.
- License check on launch: POST license key to a Cloudflare Worker (`https://license.ai-ai.app/v1/check`) which validates against the Lemon Squeezy API and returns `{ valid, expires_at, tier }`. Cache result for 7 days offline.
- Pricing recommendation for v1: **$29 USD one-time, includes 1 year of updates. $9/yr to renew updates afterward.** App keeps working forever without renewing — only updates gated.

### Error handling

| Condition | Behavior |
|---|---|
| Claude Desktop not running | Toast "Open Claude Desktop first" via tray notification. |
| UIA returns empty / chat not detected | Fallback: take a screenshot of Claude's window, send to Anthropic vision endpoint using user's key, parse out last response. Show banner "Compatibility mode — slower" in overlay so user knows. |
| API key missing | Open settings panel with API key field focused + red helper text. |
| API key invalid (401) | Toast "Invalid API key" → open settings. |
| Network error | Retry once with 500 ms backoff. Then show overlay error state with "Retry" button. |
| Rate limit (429) | Show overlay error "Anthropic rate limited — try again in a moment." |
| Hotkey already registered by another app | On settings save, show inline error "Hotkey in use by <app or unknown>" with red highlight. Old binding stays active. |
| Suggester returns malformed JSON | One auto-retry. If still malformed, show generic error. |

### Telemetry (opt-in)

Anonymous, no prompt content ever leaves the user's machine except to their own Anthropic key. Telemetry events sent to your Cloudflare Worker:

- `app_launched` (version, OS build)
- `suggestion_shown` (count)
- `suggestion_accepted` (which slot, 1–6)
- `suggestion_regenerated_via_variation` (seed value)
- `improve_used`
- `error_<type>` (anonymized type, no payload)

---

## 6. Out of scope (explicitly deferred)

- macOS support — v2.
- Web Claude (claude.ai in browser) — v2 via separate browser extension.
- Reading further back than the last assistant message — v1 only reads the most recent turn.
- Multi-conversation memory / cross-session suggestions.
- Team/shared license keys.
- Custom user-defined system prompts (only style presets in v1).
- Local LLM mode.

---

## 7. Acceptance criteria for v1

1. Installing the NSIS/WiX installer produces a tray app that auto-starts on login (configurable).
2. With Claude Desktop open and the user clicked into the chat box, `Ctrl+Shift+Space` shows the overlay above the chat input within 2 seconds (Haiku, average network).
3. Arrow keys navigate suggestions. `Ctrl+Shift+Enter` pastes the highlighted suggestion into Claude's chat box without sending (default settings).
4. With text already typed in Claude's chat box, `Ctrl+Shift+Enter` replaces it with an improved version within 2 seconds.
5. Re-pressing `Ctrl+Shift+Space` with identical context produces a visibly different set of suggestions (variation seed working).
6. API key is stored only in Windows Credential Manager; uninstall removes it.
7. App launches in under 1 second on a mid-range machine; idle memory under 80 MB.
8. License gate: trial expiry hides features, valid license unlocks them, offline cache keeps working for 7 days.
9. Telemetry off by default. With it on, no prompt content is ever transmitted.

---

## 8. Open questions for implementation planning

None blocking. The following are choices the implementation plan should make:

- Exact crate versions for `uiautomation`, `windows`, `tauri`.
- Whether to use Tauri 2.x (stable) or stay on 1.x for ecosystem maturity.
- Whether the overlay window is a Tauri secondary window or a fully separate Tauri app for tighter focus control.
- Whether the Cloudflare Worker for licensing is in scope of this repo or a separate repo.
