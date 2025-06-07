#[cfg(target_os = "linux")]
use arboard::{Clipboard, GetExtLinux, LinuxClipboardKind};
#[cfg(not(target_os = "linux"))]
use arboard::Clipboard;

fn trim_nonempty(s: String) -> Option<String> {
    let t = s.trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

/// 先 PRIMARY（划词），再标准剪贴板。Linux 用 arboard（X11/Wayland）；其它平台仅剪贴板。
pub fn read_selection_primary_then_clipboard() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let mut cb = Clipboard::new().ok()?;
        if let Ok(t) = cb
            .get()
            .clipboard(LinuxClipboardKind::Primary)
            .text()
        {
            if let Some(t) = trim_nonempty(t) {
                return Some(t);
            }
        }
        match cb
            .get()
            .clipboard(LinuxClipboardKind::Clipboard)
            .text()
        {
            Ok(t) => trim_nonempty(t),
            Err(_) => None,
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        read_clipboard_std()
    }
}

/// 仅标准剪贴板（浮层「用剪贴板再试」）。
pub fn read_clipboard_only() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let mut cb = Clipboard::new().ok()?;
        match cb
            .get()
            .clipboard(LinuxClipboardKind::Clipboard)
            .text()
        {
            Ok(t) => trim_nonempty(t),
            Err(_) => None,
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        read_clipboard_std()
    }
}

#[cfg(not(target_os = "linux"))]
fn read_clipboard_std() -> Option<String> {
    let mut cb = Clipboard::new().ok()?;
    trim_nonempty(cb.get_text().ok()?)
}
