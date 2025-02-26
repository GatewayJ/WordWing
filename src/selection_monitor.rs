// src/selection_monitor.rs
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::info;
use x11_clipboard::Clipboard;

pub struct SelectionMonitor {
    clipboard: Arc<Mutex<Clipboard>>,
    // message_rx: std::sync::mpsc::Receiver<String>,
}

impl SelectionMonitor {
    pub fn new(
        message_rx: std::sync::mpsc::Receiver<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let clipboard = Clipboard::new()?;
        let s = Self {
            clipboard: Arc::new(Mutex::new(clipboard)),
        };
        let s_clone = s.clipboard.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(message) = message_rx.try_recv() {
                    {
                        let clipboard = s_clone.lock().unwrap();
                        match clipboard.store(
                            clipboard.getter.atoms.clipboard,
                            clipboard.getter.atoms.utf8_string,
                            message.clone(),
                        ) {
                            Ok(_) => {
                                info!("Text stored to clipboard");
                            }
                            Err(err) => {
                                info!("Failed to store text to clipboard: {:?}", err);
                            }
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        Ok(s)
    }

    pub async fn get_selection_fallback(&self) -> Option<String> {
        info!("Trying fallback method with xclip");

        match Command::new("xclip")
            .args(["-out", "-selection", "primary"])
            .output()
        {
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
        if let Some(selection) = self.get_selection_main().await {
            return Some(selection);
        }
        self.get_selection_fallback().await
    }

    pub async fn get_selection_main(&self) -> Option<String> {
        info!("Attempting to read PRIMARY clipboard");

        // 增加超时时间到 500ms
        let clipboard = self.clipboard.lock().unwrap();
        match clipboard.load(
            clipboard.getter.atoms.primary,
            clipboard.getter.atoms.utf8_string,
            clipboard.getter.window,
            Duration::from_millis(5000),
        ) {
            Ok(text) => match String::from_utf8(text) {
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
            },
            Err(e) => {
                info!("Failed to load clipboard: {}", e);
                None
            }
        }
    }
}
