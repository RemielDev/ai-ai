# AI-AI — testing loop

The fastest iteration cycle for testing builds.

## One-time setup (already done on this machine)

- Rust + Cargo installed
- `cargo install tauri-cli --version "^2.0"` done

## The fast loop (no installer, ~2 min per cycle)

Just run the exe directly — no install/uninstall needed.

```bash
# 1. Rebuild
cd ~/Development/ai-ai && cargo tauri build

# 2. Wipe previous state so you start clean (optional)
rm -rf "$APPDATA/com.aiai.desktop" "$APPDATA/ai-ai"

# 3. Run
"src-tauri/target/release/ai-ai.exe"
```

Tray icon appears. To stop:

```bash
taskkill //F //IM ai-ai.exe
```

## The full-install loop (~3 min per cycle)

Use this when you want to test the actual installer experience.

```bash
# 1. Kill any running instance
taskkill //F //IM ai-ai.exe 2>/dev/null

# 2. Uninstall previous (per-user install, no admin needed)
"$LOCALAPPDATA/AI-AI/uninstall.exe" /S

# 3. Rebuild
cd ~/Development/ai-ai && cargo tauri build

# 4. Install silently
"src-tauri/target/release/bundle/nsis/AI-AI_0.1.0_x64-setup.exe" /S

# 5. Launch (auto-starts after install too)
"$LOCALAPPDATA/AI-AI/ai-ai.exe"
```

## What to actually test (in order, each cycle)

1. **First-run wizard** — appears on launch with empty state? Skip key → reaches step 3 → "Done" closes the window?
2. **Tray** — left-click opens Settings, right-click shows menu (Summon now / Settings / About / Quit)?
3. **API key** — Settings → Setup tab → paste key → "Save & verify" → green check?
4. **Hotkey recorder** — Setup tab → click a recorder → press `Ctrl+Shift+J` → captured as kbd chips?
5. **Tabs** — Behavior / Privacy / About all render? Switches and slider work?
6. **Summon** — open Claude Desktop, click into chat after a response, press `Ctrl+Shift+Space` → overlay appears above chat input?
7. **Nav** — arrow keys move highlight? `1`–`6` jump? `Esc` dismisses?
8. **Paste** — highlight a suggestion → `Ctrl+Shift+Enter` → text appears in Claude's chat box (NOT sent unless auto-send is on)?
9. **Improve** — type a draft in Claude's chat → `Ctrl+Shift+Enter` → draft is rewritten in place?
10. **Re-summon for variation** — `Ctrl+Shift+Space` again with same Claude response → suggestions are different?

## Reset between tests

Quick state wipe (no uninstall):

```bash
taskkill //F //IM ai-ai.exe 2>/dev/null
rm -rf "$APPDATA/com.aiai.desktop"
```

Then relaunch — back to first-run.

## Log location

`%APPDATA%\ai-ai\` — `crash.log` written on panic. Run from a terminal to see live tracing.
