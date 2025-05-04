// src/text_replacer.rs
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

pub struct TextReplacer;

impl TextReplacer {
    pub fn new() -> Self {
        Self
    }

    /// 使用 xdotool 替换选中的文本
    /// 原理：删除选中文本，然后输入新文本
    /// 如果文本不可编辑，会返回相应的错误
    pub async fn replace_selection(&self, new_text: &str) -> Result<(), String> {
        info!("Replacing selected text with: {}", new_text);

        // 等待一小段时间，确保弹窗已关闭且焦点回到原始窗口
        sleep(Duration::from_millis(200)).await;

        // 方法1: 使用 xdotool（如果可用）
        match self.try_replace_with_xdotool(new_text).await {
            Ok(_) => {
                // 替换操作已执行，假设成功（因为操作没有返回错误）
                // 验证可能不准确，因为替换后文本通常不再被选中
                info!("Text replacement completed via xdotool (operation executed successfully)");
                return Ok(());
            }
            Err(e) => {
                warn!("xdotool method failed: {}, trying clipboard method", e);
            }
        }

        // 方法2: 使用剪贴板 + 粘贴（备用方案，仅在 xdotool 失败时使用）
        match self.try_replace_with_clipboard(new_text).await {
            Ok(_) => {
                // 替换操作已执行，假设成功
                info!("Text replacement completed via clipboard method (operation executed successfully)");
                return Ok(());
            }
            Err(e) => {
                warn!("Clipboard method also failed: {}", e);
            }
        }

        Err("无法替换文本：当前选中的文本可能不可编辑，或者替换操作失败。请确保文本在可编辑的应用程序中（如文本编辑器、输入框等）。".to_string())
    }


    /// 使用 xdotool 替换文本
    async fn try_replace_with_xdotool(&self, new_text: &str) -> Result<(), String> {
        // 检查 xdotool 是否可用
        if Command::new("which").arg("xdotool").output().is_err() {
            return Err("xdotool not found".to_string());
        }

        // 重要：不要点击，因为点击会取消文本选择！
        // 使用 Ctrl+X 剪切选中文本（这会删除选中文本并复制到剪贴板）
        // 这比 Delete 键更可靠，因为 Ctrl+X 明确作用于选中的文本
        info!("Sending Ctrl+X to cut selected text (preserves selection context)");
        let cut_result = Command::new("xdotool")
            .arg("key")
            .arg("--clearmodifiers")
            .arg("ctrl+x")
            .output()
            .map_err(|e| format!("Failed to execute xdotool: {}", e))?;

        if !cut_result.status.success() {
            let error = String::from_utf8_lossy(&cut_result.stderr);
            warn!("Ctrl+X failed: {}, trying Delete key", error);
            // 备用方案：使用 Delete 键
            let delete_result = Command::new("xdotool")
                .arg("key")
                .arg("--clearmodifiers")
                .arg("Delete")
                .output()
                .map_err(|e| format!("Failed to execute xdotool: {}", e))?;
            
            if !delete_result.status.success() {
                let error = String::from_utf8_lossy(&delete_result.stderr);
                warn!("Delete key also failed: {}, trying Backspace", error);
                let backspace_result = Command::new("xdotool")
                    .arg("key")
                    .arg("--clearmodifiers")
                    .arg("BackSpace")
                    .output()
                    .map_err(|e| format!("Failed to execute xdotool: {}", e))?;
                if !backspace_result.status.success() {
                    let error = String::from_utf8_lossy(&backspace_result.stderr);
                    warn!("All deletion methods failed: {}", error);
                } else {
                    info!("Backspace key sent successfully");
                }
            } else {
                info!("Delete key sent successfully");
            }
        } else {
            info!("Ctrl+X (cut) sent successfully");
        }

        // 等待一小段时间确保删除完成
        sleep(Duration::from_millis(250)).await;

        // 使用剪贴板方法输入新文本（比 type 命令更可靠）
        // 因为 type 命令在某些应用中可能不工作，而 Ctrl+V 粘贴更通用
        info!("Inputting new text via clipboard method (more reliable than type command)");
        self.fallback_type_with_clipboard(new_text).await
    }

    /// 使用剪贴板替换文本（备用方案）
    /// 原理：将新文本复制到剪贴板，先删除选中文本，然后粘贴
    async fn try_replace_with_clipboard(&self, new_text: &str) -> Result<(), String> {
        // 将文本复制到剪贴板
        info!("Copying text to clipboard");
        let mut child = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to execute xclip: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(new_text.as_bytes())
                .map_err(|e| format!("Failed to write to xclip: {}", e))?;
            stdin.flush()
                .map_err(|e| format!("Failed to flush xclip: {}", e))?;
        }

        child.wait()
            .map_err(|e| format!("Failed to wait for xclip: {}", e))?;

        // 等待剪贴板更新
        sleep(Duration::from_millis(50)).await;

        // 删除选中的文本（不要点击，保持选择状态）
        info!("Sending Delete key to remove selected text");
        let delete_result = Command::new("xdotool")
            .arg("key")
            .arg("--clearmodifiers")
            .arg("Delete")
            .output();
        
        match delete_result {
            Ok(output) => {
                if !output.status.success() {
                    warn!("Delete key failed, trying Backspace");
                    let backspace_result = Command::new("xdotool")
                        .arg("key")
                        .arg("--clearmodifiers")
                        .arg("BackSpace")
                        .output()
                        .map_err(|e| format!("Failed to execute xdotool: {}", e))?;
                    if !backspace_result.status.success() {
                        let error = String::from_utf8_lossy(&backspace_result.stderr);
                        warn!("Both Delete and Backspace failed: {}", error);
                    } else {
                        info!("Backspace key sent successfully");
                    }
                } else {
                    info!("Delete key sent successfully");
                }
            }
            Err(e) => {
                warn!("Failed to send Delete key: {}", e);
            }
        }

        // 等待删除完成
        sleep(Duration::from_millis(150)).await;

        // 粘贴剪贴板内容
        info!("Pasting from clipboard");
        let paste_result = Command::new("xdotool")
            .arg("key")
            .arg("--clearmodifiers")
            .arg("ctrl+v")
            .output()
            .map_err(|e| format!("Failed to execute xdotool: {}", e))?;

        if !paste_result.status.success() {
            let error = String::from_utf8_lossy(&paste_result.stderr);
            return Err(format!("Failed to paste: the target may not be editable. Error: {}", error));
        }

        info!("Paste successful");
        Ok(())
    }

    /// 备用方法：使用剪贴板输入文本（当 type 命令失败时）
    async fn fallback_type_with_clipboard(&self, new_text: &str) -> Result<(), String> {
        info!("Using clipboard method to input text");
        
        // 将文本复制到剪贴板
        let mut child = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to execute xclip: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(new_text.as_bytes())
                .map_err(|e| format!("Failed to write to xclip: {}", e))?;
            stdin.flush()
                .map_err(|e| format!("Failed to flush xclip: {}", e))?;
        }

        child.wait()
            .map_err(|e| format!("Failed to wait for xclip: {}", e))?;

        // 等待剪贴板更新
        sleep(Duration::from_millis(50)).await;

        // 粘贴剪贴板内容
        info!("Pasting text from clipboard");
        let paste_result = Command::new("xdotool")
            .arg("key")
            .arg("--clearmodifiers")
            .arg("ctrl+v")
            .output()
            .map_err(|e| format!("Failed to execute xdotool: {}", e))?;

        if !paste_result.status.success() {
            let error = String::from_utf8_lossy(&paste_result.stderr);
            return Err(format!("Failed to paste text: {}", error));
        }

        info!("Text pasted successfully via clipboard");
        Ok(())
    }

}

