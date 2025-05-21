mod selection;
mod settings;
mod translate;
mod vocabulary;

use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use settings::AppSettings;
use vocabulary::{VocabItem, VocabStore};

/// Tauri 2：对 `WebviewWindow` 调用 `emit` 时，事件未必投递到该 label 的 Webview；
/// 使用 `AppHandle::emit_to("translate-overlay", …)` 与前端 `listen` 对齐。
fn overlay_emit(app: &AppHandle, payload: serde_json::Value) {
    if let Err(e) = app.emit_to("translate-overlay", "translate-state", payload) {
        eprintln!("[WordWing] emit_to translate-overlay failed: {}", e);
    }
}

fn overlay_bring_up(win: &tauri::WebviewWindow) {
    let _ = win.unminimize();
    let _ = win.center();
    let _ = win.show();
    let _ = win.set_focus();
}

fn register_translate_hotkey(app: &AppHandle, preset: &str) -> Result<(), String> {
    let sc = settings::preset_to_shortcut(preset)?;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    gs.on_shortcut(sc, |app, _sc, event| {
        if event.state != ShortcutState::Pressed {
            return;
        }
        let h = app.clone();
        tauri::async_runtime::spawn(async move {
            translate_flow_selection_first(h).await;
        });
    })
    .map_err(|e| e.to_string())
}

async fn translate_flow_core(app: AppHandle, source: Option<String>, clipboard_only: bool) {
    let Some(win) = app.get_webview_window("translate-overlay") else {
        eprintln!("[WordWing] missing webview window label translate-overlay");
        return;
    };

    let source = source.and_then(|s| {
        let t = s.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    });

    let Some(source) = source else {
        let reason = if clipboard_only {
            "剪贴板为空"
        } else {
            "未选中文字（PRIMARY 与剪贴板均为空）"
        };
        overlay_bring_up(&win);
        tokio::time::sleep(Duration::from_millis(50)).await;
        overlay_emit(
            &app,
            serde_json::json!({ "kind": "empty", "reason": reason }),
        );
        return;
    };

    overlay_bring_up(&win);
    tokio::time::sleep(Duration::from_millis(50)).await;

    let preview: String = source.chars().take(500).collect();
    let truncated = source.chars().count() > 500;
    overlay_emit(
        &app,
        serde_json::json!({
            "kind": "loading",
            "source": preview,
            "source_truncated": truncated
        }),
    );

    let api_key = match std::env::var("DASHSCOPE_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k,
        _ => {
            overlay_emit(
                &app,
                serde_json::json!({
                    "kind": "error",
                    "source": Some(source.chars().take(200).collect::<String>()),
                    "message": "未配置 DASHSCOPE_API_KEY。请在启动应用的终端中执行 export DASHSCOPE_API_KEY=…，或写入 ~/.bashrc / ~/.profile。"
                }),
            );
            return;
        }
    };

    let target = translate::target_language_label(&source);
    match translate::translate_dashscope(&api_key, &source, target).await {
        Ok(t) if !t.trim().is_empty() => {
            overlay_emit(
                &app,
                serde_json::json!({
                    "kind": "success",
                    "source": source,
                    "translation": t.trim(),
                    "target_lang": target
                }),
            );
        }
        Ok(_) => {
            overlay_emit(
                &app,
                serde_json::json!({
                    "kind": "error",
                    "source": Some(source),
                    "message": "暂无译文（服务返回空）"
                }),
            );
        }
        Err(e) => {
            overlay_emit(
                &app,
                serde_json::json!({
                    "kind": "error",
                    "source": Some(source.chars().take(200).collect::<String>()),
                    "message": e
                }),
            );
        }
    }
}

async fn translate_flow_selection_first(app: AppHandle) {
    let source = selection::read_selection_primary_then_clipboard();
    translate_flow_core(app, source, false).await;
}

#[tauri::command]
fn list_vocabulary(store: tauri::State<VocabStore>) -> Result<Vec<VocabItem>, String> {
    store.list()
}

#[tauri::command]
fn add_vocabulary_item(
    app: AppHandle,
    store: tauri::State<VocabStore>,
    source_text: String,
    translation: String,
    target_lang: String,
) -> Result<VocabItem, String> {
    let item = store.add(source_text, translation, target_lang)?;
    let _ = app.emit("vocabulary-changed", ());
    Ok(item)
}

#[tauri::command]
fn delete_vocabulary_item(
    app: AppHandle,
    store: tauri::State<VocabStore>,
    id: String,
) -> Result<(), String> {
    store.remove(&id)?;
    let _ = app.emit("vocabulary-changed", ());
    Ok(())
}

#[tauri::command]
async fn translate_from_clipboard_only(app: AppHandle) -> Result<(), String> {
    let source = selection::read_clipboard_only();
    translate_flow_core(app, source, true).await;
    Ok(())
}

/// 浮层「重试」：不重新取选区，仅对已知原文再次请求翻译。
#[tauri::command]
async fn retry_translate_with_text(app: AppHandle, source: String) -> Result<(), String> {
    translate_flow_core(app, Some(source), false).await;
    Ok(())
}

#[tauri::command]
fn get_translate_hotkey_preset(state: tauri::State<AppSettings>) -> String {
    state.preset()
}

#[tauri::command]
fn get_translate_hotkey_display(state: tauri::State<AppSettings>) -> String {
    settings::preset_display_label(&state.preset()).to_string()
}

#[tauri::command]
fn list_translate_hotkey_choices() -> Vec<settings::HotkeyChoice> {
    settings::all_hotkey_choices()
}

#[tauri::command]
fn set_translate_hotkey_preset(
    app: AppHandle,
    state: tauri::State<AppSettings>,
    preset: String,
) -> Result<(), String> {
    state.set_preset(&preset)?;
    register_translate_hotkey(&app, &preset)?;
    let _ = app.emit("translate-hotkey-changed", ());
    Ok(())
}

/// 与全局热键相同逻辑；可从主窗口按钮调用（Wayland 下全局快捷键常不可用）。
#[tauri::command]
async fn trigger_translate_overlay(app: AppHandle) -> Result<(), String> {
    translate_flow_selection_first(app).await;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let global_shortcut_plugin = tauri_plugin_global_shortcut::Builder::new().build();

    tauri::Builder::default()
        .plugin(global_shortcut_plugin)
        .setup(|app| {
            let dir = app.path().app_data_dir().map_err(|e| -> Box<dyn std::error::Error> {
                e.to_string().into()
            })?;
            std::fs::create_dir_all(&dir).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let path = dir.join("vocabulary.json");
            let store = VocabStore::load(path).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            app.manage(store);
            let app_settings = AppSettings::load(&dir).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let preset = app_settings.preset();
            app.manage(app_settings);
            register_translate_hotkey(&app.handle(), &preset)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_vocabulary,
            add_vocabulary_item,
            delete_vocabulary_item,
            translate_from_clipboard_only,
            retry_translate_with_text,
            get_translate_hotkey_preset,
            get_translate_hotkey_display,
            list_translate_hotkey_choices,
            set_translate_hotkey_preset,
            trigger_translate_overlay
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
