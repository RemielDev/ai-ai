//! ClaudeReader — Windows UI Automation extraction of Claude Desktop chat.
//!
//! Strategy:
//! 1. Find Claude Desktop window via process executable name (`Claude.exe`).
//! 2. Walk UIA tree to collect text from message-bearing elements.
//! 3. Locate the chat input (the bottom-most Edit / Document element).
//! 4. Return a `ChatSnapshot`.
//!
//! The tree walk is heuristic by necessity — Electron apps don't expose
//! semantic roles for chat-turn boundaries. We collect text nodes in
//! tree order and treat the tail as the most recent assistant turn.
//! When the heuristic fails (empty assistant_msg), the overlay shows a
//! friendly error and asks the user to retry.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use uiautomation::controls::ControlType;
use uiautomation::types::UIProperty;
use uiautomation::{UIAutomation, UIElement, UITreeWalker};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChatSnapshot {
    pub last_assistant_msg: String,
    pub last_user_msg: Option<String>,
    pub chat_input_text: String,
    pub chat_input_bounds: Rect,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

fn is_null_hwnd(h: &HWND) -> bool {
    h.0.is_null()
}

/// True if the foreground window belongs to Claude Desktop (Claude.exe).
pub fn is_claude_focused() -> bool {
    foreground_process_name()
        .map(|n| n.eq_ignore_ascii_case("Claude.exe"))
        .unwrap_or(false)
}

/// True if foreground window belongs to AI-AI itself.
#[allow(dead_code)]
pub fn is_self_focused() -> bool {
    foreground_process_name()
        .map(|n| n.eq_ignore_ascii_case("ai-ai.exe"))
        .unwrap_or(false)
}

/// Heuristic for Flow C: is the user's caret inside Claude's chat input?
pub fn is_chat_input_focused() -> Result<bool> {
    if !is_claude_focused() {
        return Ok(false);
    }
    let auto = UIAutomation::new().context("init UIAutomation")?;
    let Ok(focused) = auto.get_focused_element() else {
        return Ok(false);
    };
    let ctype = focused.get_control_type().unwrap_or(ControlType::Custom);
    Ok(matches!(ctype, ControlType::Edit | ControlType::Document))
}

/// Build a snapshot of Claude's current conversation state.
pub fn snapshot() -> Result<ChatSnapshot> {
    let auto = UIAutomation::new().context("init UIAutomation")?;
    let hwnd = unsafe { GetForegroundWindow() };
    if is_null_hwnd(&hwnd) {
        anyhow::bail!("no foreground window");
    }

    let root = auto
        .element_from_handle((hwnd.0 as isize).into())
        .context("element from hwnd")?;
    let walker = auto.get_control_view_walker().context("get walker")?;

    let mut text_chunks: Vec<String> = Vec::new();
    let mut bottom_edit: Option<(String, Rect, i32)> = None;
    walk(
        &root,
        &walker,
        &mut text_chunks,
        &mut bottom_edit,
        0,
    );

    // Assemble the assistant message from the tail of the collected text.
    let tail_iter = text_chunks
        .iter()
        .rev()
        .filter(|s| {
            let t = s.trim();
            !t.is_empty() && t.len() > 1
        });

    let mut user_msg: Option<String> = None;
    let mut assistant_chunks: Vec<String> = Vec::new();
    for chunk in tail_iter {
        if assistant_chunks.len() > 6
            && chunk.split_whitespace().count() < 12
            && user_msg.is_none()
        {
            user_msg = Some(chunk.clone());
            break;
        }
        assistant_chunks.push(chunk.clone());
        if assistant_chunks.len() > 30 {
            break;
        }
    }
    assistant_chunks.reverse();
    let assistant_msg = assistant_chunks.join("\n").trim().to_string();

    let (chat_input_text, chat_input_bounds) = bottom_edit
        .map(|(t, r, _)| (t, r))
        .unwrap_or_default();

    Ok(ChatSnapshot {
        last_assistant_msg: assistant_msg,
        last_user_msg: user_msg,
        chat_input_text,
        chat_input_bounds,
    })
}

fn walk(
    element: &UIElement,
    walker: &UITreeWalker,
    text_chunks: &mut Vec<String>,
    bottom_edit: &mut Option<(String, Rect, i32)>,
    depth: usize,
) {
    if depth > 60 {
        return;
    }

    let ctype = element.get_control_type().unwrap_or(ControlType::Custom);

    if matches!(ctype, ControlType::Edit | ControlType::Document) {
        let value = element
            .get_property_value(UIProperty::ValueValue)
            .ok()
            .and_then(|v| v.get_string().ok())
            .unwrap_or_default();
        let bounds = element
            .get_bounding_rectangle()
            .ok()
            .map(|r| Rect {
                x: r.get_left(),
                y: r.get_top(),
                width: r.get_right() - r.get_left(),
                height: r.get_bottom() - r.get_top(),
            })
            .unwrap_or_default();
        // Track the lowest editable element on screen.
        let bottom_y = bounds.y + bounds.height;
        if bottom_edit
            .as_ref()
            .map(|(_, _, y)| bottom_y >= *y)
            .unwrap_or(true)
        {
            *bottom_edit = Some((value, bounds, bottom_y));
        }
    } else {
        // Collect element names that carry message text.
        if let Ok(name) = element.get_name() {
            let trimmed = name.trim();
            if trimmed.len() > 3 {
                text_chunks.push(trimmed.to_string());
            }
        }
    }

    if let Ok(first) = walker.get_first_child(element) {
        let mut current = Some(first);
        while let Some(child) = current {
            walk(&child, walker, text_chunks, bottom_edit, depth + 1);
            current = walker.get_next_sibling(&child).ok();
        }
    }
}

fn foreground_process_name() -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    unsafe {
        let hwnd = GetForegroundWindow();
        if is_null_hwnd(&hwnd) {
            return None;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let len = GetModuleFileNameExW(handle, None, &mut buf);
        let _ = CloseHandle(handle);
        if len == 0 {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        full.rsplit(['\\', '/']).next().map(|s| s.to_string())
    }
}

/// HWND of the foreground window.
#[allow(dead_code)]
pub fn foreground_hwnd() -> Option<HWND> {
    unsafe {
        let h = GetForegroundWindow();
        if is_null_hwnd(&h) {
            None
        } else {
            Some(h)
        }
    }
}
