# AI-AI — testing loop (PowerShell)

The fastest iteration cycle. Copy-paste each block into PowerShell.

## Fast loop — no installer (~2 min per cycle)

```powershell
cd ~/Development/ai-ai
cargo tauri build
& "src-tauri\target\release\ai-ai.exe"
```

Tray icon appears. To stop:

```powershell
taskkill /F /IM ai-ai.exe
```

## Reset to first-run state (keeps build, wipes settings + API key)

```powershell
taskkill /F /IM ai-ai.exe 2>$null
Remove-Item -Recurse -Force "$env:APPDATA\com.aiai.desktop" -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force "$env:APPDATA\ai-ai" -ErrorAction SilentlyContinue
```

Then relaunch — onboarding wizard fires again.

## Full installer loop (~3 min)

```powershell
taskkill /F /IM ai-ai.exe 2>$null
& "$env:LOCALAPPDATA\AI-AI\uninstall.exe" /S
cd ~/Development/ai-ai
cargo tauri build
& "src-tauri\target\release\bundle\nsis\AI-AI_0.1.0_x64-setup.exe" /S
& "$env:LOCALAPPDATA\AI-AI\ai-ai.exe"
```

## What to test (10-step checklist)

1. **First-run wizard** — appears on launch? Skip key → reaches step 3 → "Done" closes the window?
2. **Tray** — left-click opens Settings, right-click shows menu (Summon now / Settings / About / Quit)?
3. **API key** — Settings → Setup tab → paste key → "Save & verify" → green check?
4. **Hotkey recorder** — Setup tab → click a recorder → press `Ctrl+Shift+J` → captured as kbd chips?
5. **Tabs** — Behavior / Privacy / About all render? Switches and slider work?
6. **Summon** — open Claude Desktop, click into chat after a response, press `Ctrl+Shift+Space` → overlay appears above chat input?
7. **Nav** — arrow keys move highlight? `1`–`6` jump? `Esc` dismisses?
8. **Paste** — highlight → `Ctrl+Shift+Enter` → text appears in Claude's chat box (NOT sent unless auto-send is on)?
9. **Improve** — type a draft in Claude's chat → `Ctrl+Shift+Enter` → draft is rewritten in place?
10. **Re-summon** — `Ctrl+Shift+Space` again with same Claude response → suggestions differ from last batch?

## Log location

`%APPDATA%\ai-ai\crash.log` written on panic. Run from a terminal to see live tracing.

## PowerShell gotchas

| bash | PowerShell |
|---|---|
| `cmd1 && cmd2` | separate lines, or `;` |
| `"path/to.exe"` | `& "path\to.exe"` |
| `taskkill //F //IM x` | `taskkill /F /IM x` |
| `2>/dev/null` | `2>$null` |
| `rm -rf X` | `Remove-Item -Recurse -Force X` |
