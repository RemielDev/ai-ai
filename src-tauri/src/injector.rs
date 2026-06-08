//! Injector - pastes text into Claude Desktop's chat input.
//!
//! Strategy:
//! 1. Try the focused UIA Edit element's `ValuePattern.SetValue`. Fast and
//!    doesn't disturb the clipboard.
//! 2. Fall back to clipboard set + simulated Ctrl+V.
//! 3. Optionally simulate Enter when `auto_send` is true.

use anyhow::{Context, Result};
use std::time::Duration;
use tokio::time::sleep;

use uiautomation::controls::ControlType;
use uiautomation::patterns::UIValuePattern;
use uiautomation::UIAutomation;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_CONTROL, VK_RETURN, VK_V,
};

pub async fn paste(text: &str, auto_send: bool) -> Result<()> {
    // Try direct UIA SetValue first.
    let direct_ok = match try_set_value_direct(text) {
        Ok(()) => true,
        Err(e) => {
            tracing::debug!(?e, "direct SetValue failed, falling back to clipboard");
            false
        }
    };

    if !direct_ok {
        clipboard_paste(text).await?;
    }

    if auto_send {
        sleep(Duration::from_millis(60)).await;
        send_enter()?;
    }
    Ok(())
}

fn try_set_value_direct(text: &str) -> Result<()> {
    let auto = UIAutomation::new().context("UIAutomation::new")?;
    let focused = auto.get_focused_element().context("focused element")?;
    let ctype = focused.get_control_type().unwrap_or(ControlType::Custom);
    if !matches!(ctype, ControlType::Edit | ControlType::Document) {
        anyhow::bail!("focused element is not editable");
    }
    let pattern: UIValuePattern = focused
        .get_pattern::<UIValuePattern>()
        .context("get ValuePattern")?;
    pattern.set_value(text).context("set value")?;
    Ok(())
}

async fn clipboard_paste(text: &str) -> Result<()> {
    use arboard::Clipboard;

    // Save and restore prior clipboard content where possible.
    let mut clipboard = Clipboard::new().context("open clipboard")?;
    let prior = clipboard.get_text().ok();
    clipboard.set_text(text.to_string()).context("set clipboard text")?;
    // Small delay so the clipboard latches before SendInput fires.
    sleep(Duration::from_millis(40)).await;
    send_ctrl_v()?;
    sleep(Duration::from_millis(80)).await;
    if let Some(prev) = prior {
        let _ = clipboard.set_text(prev);
    }
    Ok(())
}

fn send_ctrl_v() -> Result<()> {
    send_keys(&[
        (VK_CONTROL, false),
        (VK_V, false),
        (VK_V, true),
        (VK_CONTROL, true),
    ])
}

fn send_enter() -> Result<()> {
    send_keys(&[(VK_RETURN, false), (VK_RETURN, true)])
}

fn send_keys(keys: &[(VIRTUAL_KEY, bool)]) -> Result<()> {
    let mut inputs: Vec<INPUT> = Vec::with_capacity(keys.len());
    for (key, is_up) in keys {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: *key,
                    wScan: 0,
                    dwFlags: if *is_up {
                        KEYEVENTF_KEYUP
                    } else {
                        KEYBD_EVENT_FLAGS(0)
                    },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        anyhow::bail!("SendInput sent {} of {} events", sent, inputs.len());
    }
    Ok(())
}
