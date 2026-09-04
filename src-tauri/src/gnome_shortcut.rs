//! GNOME/Wayland 没有实现 GlobalShortcuts Portal 时的替代方案。
//!
//! GNOME Shell 自身的 custom-keybindings 在 Wayland 下可以可靠接收全局按键；
//! 绑定通过 `systemctl --user kill` 向当前 user service 的主进程发送信号，
//! 应用内的 Tokio 信号监听器再触发对应翻译流程。

use gio::prelude::{SettingsExt, SettingsExtManual};
use tauri::AppHandle;
use tokio::signal::unix::{signal, SignalKind};

const ROOT_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const BINDING_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
const ROOT_KEY: &str = "custom-keybindings";
const MAIN_PATH: &str =
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/wordwing-translate/";
const BILINGUAL_PATH: &str =
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/wordwing-bilingual/";
const MAIN_COMMAND: &str =
    "systemctl --user kill --kill-whom=main --signal=SIGUSR1 wordwing.service";
const BILINGUAL_COMMAND: &str =
    "systemctl --user kill --kill-whom=main --signal=SIGUSR2 wordwing.service";

pub fn should_use() -> bool {
    // 绑定命令明确面向官方 systemd user service；开发模式继续走插件/Portal，
    // 避免快捷键误触发另一个已安装实例。
    if std::env::var_os("WAYLAND_DISPLAY").is_none() || std::env::var_os("INVOCATION_ID").is_none()
    {
        return false;
    }
    ["XDG_CURRENT_DESKTOP", "DESKTOP_SESSION"]
        .iter()
        .filter_map(|key| std::env::var(key).ok())
        .any(|value| {
            let value = value.to_ascii_lowercase();
            value.contains("gnome") || value.contains("ubuntu")
        })
}

pub fn install(preset: &str) -> Result<(), String> {
    let trigger = crate::settings::preset_to_portal_preferred_trigger(preset)
        .ok_or_else(|| format!("GNOME 无法表达快捷键预设: {preset}"))?;

    let root = gio::Settings::new(ROOT_SCHEMA);
    let mut paths: Vec<String> = root
        .strv(ROOT_KEY)
        .iter()
        .map(|path| path.to_string())
        .collect();
    for path in [MAIN_PATH, BILINGUAL_PATH] {
        if !paths.iter().any(|existing| existing == path) {
            paths.push(path.to_string());
        }
    }

    let main = gio::Settings::with_path(BINDING_SCHEMA, MAIN_PATH);
    main.set_string("name", "WordWing：划词或剪贴板翻译")
        .map_err(|e| e.to_string())?;
    main.set_string("command", MAIN_COMMAND)
        .map_err(|e| e.to_string())?;
    main.set_string("binding", trigger)
        .map_err(|e| e.to_string())?;

    let bilingual = gio::Settings::with_path(BINDING_SCHEMA, BILINGUAL_PATH);
    bilingual
        .set_string("name", "WordWing：中英翻译")
        .map_err(|e| e.to_string())?;
    bilingual
        .set_string("command", BILINGUAL_COMMAND)
        .map_err(|e| e.to_string())?;
    bilingual
        .set_string("binding", "<Control><Shift>2")
        .map_err(|e| e.to_string())?;

    let path_refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    root.set_strv(ROOT_KEY, path_refs.as_slice())
        .map_err(|e| e.to_string())?;
    gio::Settings::sync();
    Ok(())
}

pub fn spawn_signal_loop(app: AppHandle) {
    // Tauri 的 setup 回调运行在主线程，不能在这里直接创建 Tokio signal stream；
    // stream 必须在 Tauri async runtime 的上下文内初始化。
    tauri::async_runtime::spawn(async move {
        let mut normal = match signal(SignalKind::user_defined1()) {
            Ok(signal) => signal,
            Err(error) => {
                eprintln!("[WordWing] 无法监听 GNOME 普通翻译快捷键信号: {error}");
                return;
            }
        };
        let mut bilingual = match signal(SignalKind::user_defined2()) {
            Ok(signal) => signal,
            Err(error) => {
                eprintln!("[WordWing] 无法监听 GNOME 中英翻译快捷键信号: {error}");
                return;
            }
        };
        loop {
            tokio::select! {
                received = normal.recv() => {
                    if received.is_none() {
                        return;
                    }
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        crate::translate_flow_selection_first(app).await;
                    });
                }
                received = bilingual.recv() => {
                    if received.is_none() {
                        return;
                    }
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        crate::translate_zh_en_selection_first(app).await;
                    });
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn gnome_commands_target_only_the_service_main_process() {
        assert!(super::MAIN_COMMAND.contains("--kill-whom=main"));
        assert!(super::MAIN_COMMAND.contains("SIGUSR1"));
        assert!(super::BILINGUAL_COMMAND.contains("SIGUSR2"));
    }
}
