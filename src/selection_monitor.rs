// src/selection_monitor.rs
use x11_clipboard::Clipboard;
use std::time::Duration;
use tracing::info;
use std::process::Command;

pub struct SelectionMonitor {
    clipboard: Clipboard,
}

impl SelectionMonitor {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let clipboard = Clipboard::new()?;
        Ok(Self { clipboard })
    }

        pub async fn get_selection_fallback(&self) -> Option<String> {
        info!("Trying fallback method with xclip");
        
        match Command::new("xclip")
            .args(["-out", "-selection", "primary"])
            .output() {
            Ok(output) => {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !text.is_empty() {
                        Some(text)
                    } else {
                        None
                    }
                } else {
                    info!("xclip command failed");
                    None
                }
            }
            Err(e) => {
                info!("Failed to execute xclip: {}", e);
                None
            }
        }
    }
    pub async fn get_selection(&self) -> Option<String> {
        if let Some(selection) =  self.get_selection_main().await{
            return Some(selection)
        }
        self.get_selection_fallback().await
    }

    pub async fn get_selection_main(&self) -> Option<String> {
        info!("Attempting to read PRIMARY clipboard");
        
        // 增加超时时间到 500ms
        match self.clipboard.load(
            self.clipboard.getter.atoms.primary,
            self.clipboard.getter.atoms.utf8_string,
            self.clipboard.getter.window,
            Duration::from_millis(5000),
        ) {
            Ok(text) => {
                match String::from_utf8(text) {
                    Ok(text) => {
                        let trimmed = text.trim().to_string();
                        if !trimmed.is_empty() {
                            Some(trimmed)
                        } else {
                            info!("Clipboard content is empty");
                            None
                        }
                    }
                    Err(e) => {
                        info!("Failed to decode clipboard content as UTF-8: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                info!("Failed to load clipboard: {}", e);
                None
            }
        }
    }
}