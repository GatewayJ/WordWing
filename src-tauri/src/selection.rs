use std::process::Command;

/// X11：先 PRIMARY（划词），再 clipboard（兜底）。无 xclip 时返回 None。
pub fn read_selection_primary_then_clipboard() -> Option<String> {
    if let Some(t) = xclip_out("primary") {
        return Some(t);
    }
    xclip_out("clipboard")
}

/// 仅剪贴板（浮层「用剪贴板再试」）。
pub fn read_clipboard_only() -> Option<String> {
    xclip_out("clipboard")
}

fn xclip_out(selection: &str) -> Option<String> {
    let output = Command::new("xclip")
        .args(["-o", "-selection", selection])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
