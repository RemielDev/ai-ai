# AI-AI

Prompt copilot for Claude Desktop on Windows.

Two hotkeys:

- **`Ctrl+Shift+Space`** — read Claude's most recent response, show 3–6 AI-generated follow-up prompts in a floating overlay above the chat input.
- **`Ctrl+Shift+Enter`** — context-aware:
  - In the overlay → paste the highlighted suggestion into Claude's chat box (don't send by default).
  - In Claude's chat box → improve the prompt you've drafted, in place.

Bring your own Anthropic API key. AI-AI never proxies your prompts.

## Build (developer setup)

Requirements:
- Windows 10/11
- Rust 1.77+ (`rustup install stable`)
- Microsoft C++ Build Tools (MSVC)
- WebView2 Runtime (preinstalled on Windows 11)
- Node.js 18+ (only for the Tauri CLI; UI is static)
- `cargo install tauri-cli --version "^2.0"`

Run dev:
```
cargo tauri dev
```

Build installer:
```
cargo tauri build
```

Installer lands in `src-tauri/target/release/bundle/nsis/`.

## Architecture

See `docs/superpowers/specs/2026-05-24-ai-ai-design.md` for the full design.

Five units, each one file:

| File | Job |
|---|---|
| `src-tauri/src/hotkey.rs` | Register global hotkeys, dispatch by focused window. |
| `src-tauri/src/reader.rs` | Walk Claude Desktop's UIA tree, extract chat snapshot. |
| `src-tauri/src/suggester.rs` | Anthropic API calls — followups + improve. |
| `src-tauri/src/injector.rs` | Paste text into Claude's chat input (UIA + clipboard fallback). |
| `src-tauri/src/lib.rs` + frontend in `ui/` | Tauri tray + overlay + settings windows. |

## License & pricing (planned)

Sold via Lemon Squeezy. v1: $29 one-time, includes 1 year of updates. v0.1 ships with the license gate in dev-bypass mode.
