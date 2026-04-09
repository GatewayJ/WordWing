//! Wayland 下 `tauri-plugin-global-shortcut` 往往收不到按键，改用 XDG Desktop Portal 的 GlobalShortcuts。

use futures_util::StreamExt;
use std::time::Duration;
use tauri::AppHandle;

use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut};
use ashpd::desktop::CreateSessionOptions;

const PORTAL_SHORTCUT_ID: &str = "wordwing-translate";
const PORTAL_SHORTCUT_ZH_EN_ID: &str = "wordwing-zh-en";

/// 供设置页在更换预设时通知 Portal 会话重建绑定。
pub struct HotkeyPresetWatch(pub tokio::sync::watch::Sender<String>);

pub async fn portal_hotkey_loop(app: AppHandle, mut rx: tokio::sync::watch::Receiver<String>) {
    loop {
        let preset = (*rx.borrow()).clone();
        let Some(trigger) = crate::settings::preset_to_portal_preferred_trigger(&preset) else {
            eprintln!("[WordWing] 当前热键预设无法在 Wayland Portal 中表达，请换用其它预设。");
            if rx.changed().await.is_err() {
                return;
            }
            continue;
        };

        let Ok(portal) = GlobalShortcuts::new().await else {
            eprintln!(
                "[WordWing] Wayland：无法连接桌面门户 GlobalShortcuts（常见原因：未安装 xdg-desktop-portal、\
                 或当前不在图形桌面会话的 DBus 下）。已停止重试以免刷屏。\
                 不依赖门户时：请使用 X11 会话登录，或在生词页使用「打开翻译浮层（划词）」。"
            );
            return;
        };

        let Ok(session) = portal.create_session(CreateSessionOptions::default()).await else {
            eprintln!("[WordWing] GlobalShortcuts：CreateSession 失败");
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        };

        let shortcut_main = NewShortcut::new(PORTAL_SHORTCUT_ID, "WordWing：划词或剪贴板翻译")
            .preferred_trigger(Some(trigger));
        let shortcut_zh_en = NewShortcut::new(
            PORTAL_SHORTCUT_ZH_EN_ID,
            "WordWing：中英翻译（划词/剪贴板，可复制译文）",
        )
        .preferred_trigger(Some("<Control><Shift>2"));

        let bind_req = match portal
            .bind_shortcuts(
                &session,
                &[shortcut_main, shortcut_zh_en],
                None,
                BindShortcutsOptions::default(),
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[WordWing] GlobalShortcuts：BindShortcuts 失败: {}", e);
                let _ = session.close().await;
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        if bind_req.response().is_err() {
            eprintln!(
                "[WordWing] 未在系统对话框中完成全局快捷键授权。可在设置中更换热键预设后重试。"
            );
            let _ = session.close().await;
            if rx.changed().await.is_err() {
                return;
            }
            continue;
        }

        let mut activated = match portal.receive_activated().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[WordWing] GlobalShortcuts：订阅 Activated 失败: {}", e);
                let _ = session.close().await;
                continue;
            }
        };

        loop {
            tokio::select! {
                res = rx.changed() => {
                    if res.is_err() {
                        let _ = session.close().await;
                        return;
                    }
                    if *rx.borrow() != preset {
                        let _ = session.close().await;
                        break;
                    }
                }
                evt = activated.next() => {
                    let Some(act) = evt else {
                        let _ = session.close().await;
                        break;
                    };
                    let h = app.clone();
                    match act.shortcut_id() {
                        PORTAL_SHORTCUT_ID => {
                            tauri::async_runtime::spawn(async move {
                                crate::translate_flow_selection_first(h).await;
                            });
                        }
                        PORTAL_SHORTCUT_ZH_EN_ID => {
                            tauri::async_runtime::spawn(async move {
                                crate::translate_zh_en_selection_first(h).await;
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
