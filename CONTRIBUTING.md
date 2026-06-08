# Contributing to AI-AI

Thanks for reading. The codebase is small and unit-isolated - most contributions touch one file.

## Setup

Requirements: Windows 10/11, Rust 1.77+ (`rustup install stable`), MSVC build tools, Node.js 18+ for the mockup harness, WebView2 Runtime (preinstalled on Windows 11).

```powershell
git clone https://github.com/RemielDev/ai-ai
cd ai-ai
cargo install tauri-cli --version "^2.0"
cargo tauri build
```

Installer lands in `src-tauri/target/release/bundle/nsis/`.

## Dev loop

Fastest iteration cycle - no installer:

```powershell
cargo tauri build
& "src-tauri\target\release\ai-ai.exe"
# when done:
taskkill /F /IM ai-ai.exe
```

See [`TESTING.md`](TESTING.md) for the full 10-step manual checklist.

## UI mockups

```powershell
node dev-preview/snap.mjs
```

Regenerates the 12 PNGs in `mockups/`. Run after any UI change so reviewers can see what changed.

## Code layout

| File | Owns |
|---|---|
| `src-tauri/src/hotkey.rs` | Global hotkey registration + dispatch + rate limiting |
| `src-tauri/src/reader.rs` | Windows UI Automation tree walking |
| `src-tauri/src/suggester.rs` | Per-provider API dispatch + JSON normalization |
| `src-tauri/src/injector.rs` | Paste into Claude's chat box |
| `src-tauri/src/commands.rs` | Tauri IPC commands |
| `src-tauri/src/lib.rs` | Tauri builder, tray, window management |
| `ui/style.css` | Design tokens + components |
| `ui/icons.js` | SVG icon library |
| `ui/overlay.html / .js` | Suggestion overlay |
| `ui/settings.html / .js` | Settings panel (4 tabs) |
| `ui/onboarding.html / .js` | First-run wizard (4 steps) |

## Before opening a PR

- Run `cargo check` (no errors, no new warnings).
- Re-run `node dev-preview/snap.mjs` and commit any visual diff in `mockups/`.
- Keep changes scoped - one feature or fix per PR.
- Follow the existing logging style (`tracing::info!` / `warn!` with `?e` for errors).
- For UI changes, ensure WCAG AA contrast and that `prefers-reduced-motion` works.

## Reporting bugs

Open an issue with:

- Windows version (run `winver`)
- Claude Desktop version
- AI-AI version (Settings → About tab)
- Steps to reproduce
- What you expected vs what happened
- Relevant lines from `%APPDATA%\ai-ai\crash.log` if it crashed

## Provider integration questions

Adding a new provider means adding:

1. A `Provider` enum variant + entry in `key_prefix_hint()` / `console_url()` / `label()` / `slug()` in `settings.rs`
2. A call method in `suggester.rs` that maps the system+user prompts to that provider's endpoint shape
3. A dispatch arm in `Suggester::call`
4. An entry in `ui/providers.js` for the UI side

Open an issue first so we can talk through the shape - some providers have quirks worth catching before code.

## License

By contributing, you agree your code is released under the [MIT License](LICENSE).
