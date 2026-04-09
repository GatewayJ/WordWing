mod recent_translations;
mod selection;
mod settings;
mod storage;
mod todo;
mod todo_notify;
mod translate;
mod vocabulary;
#[cfg(target_os = "linux")]
mod wayland_shortcut;
mod weekly_article;

use recent_translations::{RecentTranslationsPage, RecentTranslationsStore};
use settings::AppSettings;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use todo::{TodoItem, TodoSchedule, TodoStore};
use vocabulary::{VocabItem, VocabStore};
use weekly_article::{SavedArticle, WeeklyArticleStore, WeeklyStatusDto};

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

/// 普通翻译（设置中的预设）+ 固定 **Ctrl+Shift+2** 中英翻译（划词/剪贴板，与主流程相同取词顺序）。
fn register_translate_hotkeys(app: &AppHandle, preset: &str) -> Result<(), String> {
    let sc = settings::preset_to_shortcut(preset)?;
    let sc_zh_en = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit2);
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
    .map_err(|e| e.to_string())?;
    gs.on_shortcut(sc_zh_en, |app, _sc, event| {
        if event.state != ShortcutState::Pressed {
            return;
        }
        let h = app.clone();
        tauri::async_runtime::spawn(async move {
            translate_zh_en_selection_first(h).await;
        });
    })
    .map_err(|e| e.to_string())?;
    Ok(())
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
            let translation = t.trim().to_string();
            if let Some(recent) = app.try_state::<RecentTranslationsStore>() {
                let _ = recent.push(source.clone(), translation.clone(), target.to_string());
                let _ = app.emit("recent-translations-changed", ());
            }
            overlay_emit(
                &app,
                serde_json::json!({
                    "kind": "success",
                    "source": source,
                    "translation": translation,
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

/// 划词后立刻按全局键时，部分环境下 PRIMARY 尚未就绪；短暂等待后再读一次。
async fn read_selection_primary_then_clipboard_with_retry() -> Option<String> {
    if let Some(s) = selection::read_selection_primary_then_clipboard() {
        return Some(s);
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    selection::read_selection_primary_then_clipboard()
}

pub(crate) async fn translate_flow_selection_first(app: AppHandle) {
    let source = read_selection_primary_then_clipboard_with_retry().await;
    translate_flow_core(app, source, false).await;
}

/// Ctrl+Shift+2：与主翻译相同的**划词优先**（PRIMARY → 剪贴板）与**中英互译**判断，成功后可一键复制译文。
async fn translate_bilingual_hotkey_flow_core(
    app: AppHandle,
    source: Option<String>,
    clipboard_only: bool,
) {
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
            "未选中文字：请先划词（Linux 为 PRIMARY 选区）或复制到剪贴板后再试"
        };
        overlay_bring_up(&win);
        tokio::time::sleep(Duration::from_millis(50)).await;
        overlay_emit(
            &app,
            serde_json::json!({ "kind": "empty", "reason": reason, "bilingual_overlay": true }),
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
            "source_truncated": truncated,
            "bilingual_overlay": true
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
                    "message": "未配置 DASHSCOPE_API_KEY。请在启动应用的终端中执行 export DASHSCOPE_API_KEY=…，或写入 ~/.bashrc / ~/.profile。",
                    "bilingual_overlay": true
                }),
            );
            return;
        }
    };

    let target = translate::target_language_label(&source);
    match translate::translate_dashscope(&api_key, &source, target).await {
        Ok(t) if !t.trim().is_empty() => {
            let translation = t.trim().to_string();
            overlay_emit(
                &app,
                serde_json::json!({
                    "kind": "success",
                    "source": source,
                    "translation": translation,
                    "target_lang": target,
                    "bilingual_overlay": true
                }),
            );
        }
        Ok(_) => {
            overlay_emit(
                &app,
                serde_json::json!({
                    "kind": "error",
                    "source": Some(source),
                    "message": "暂无译文（服务返回空）",
                    "bilingual_overlay": true
                }),
            );
        }
        Err(e) => {
            overlay_emit(
                &app,
                serde_json::json!({
                    "kind": "error",
                    "source": Some(source.chars().take(200).collect::<String>()),
                    "message": e,
                    "bilingual_overlay": true
                }),
            );
        }
    }
}

pub(crate) async fn translate_zh_en_selection_first(app: AppHandle) {
    let source = read_selection_primary_then_clipboard_with_retry().await;
    translate_bilingual_hotkey_flow_core(app, source, false).await;
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
    starred: Option<bool>,
) -> Result<VocabItem, String> {
    let item = store.add(
        source_text,
        translation,
        target_lang,
        starred.unwrap_or(false),
    )?;
    let _ = app.emit("vocabulary-changed", ());
    Ok(item)
}

#[tauri::command]
fn list_recent_translations_page(
    store: tauri::State<RecentTranslationsStore>,
    page: u32,
    per_page: u32,
) -> Result<RecentTranslationsPage, String> {
    store.list_page(page, per_page)
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
fn set_vocabulary_starred(
    app: AppHandle,
    store: tauri::State<VocabStore>,
    id: String,
    starred: bool,
) -> Result<(), String> {
    store.set_starred(&id, starred)?;
    let _ = app.emit("vocabulary-changed", ());
    Ok(())
}

#[tauri::command]
fn record_vocab_review(
    app: AppHandle,
    store: tauri::State<VocabStore>,
    id: String,
    remembered: bool,
) -> Result<(), String> {
    store.record_review(&id, remembered)?;
    let _ = app.emit("vocabulary-changed", ());
    Ok(())
}

#[tauri::command]
async fn translate_from_clipboard_only(app: AppHandle) -> Result<(), String> {
    let source = selection::read_clipboard_only();
    translate_flow_core(app, source, true).await;
    Ok(())
}

#[tauri::command]
async fn translate_from_clipboard_zh_en(app: AppHandle) -> Result<(), String> {
    let source = selection::read_clipboard_only();
    translate_bilingual_hotkey_flow_core(app, source, true).await;
    Ok(())
}

/// 浮层「重试」：不重新取选区，仅对已知原文再次请求翻译。
#[tauri::command]
async fn retry_translate_with_text(app: AppHandle, source: String) -> Result<(), String> {
    translate_flow_core(app, Some(source), false).await;
    Ok(())
}

#[tauri::command]
async fn retry_translate_zh_en_with_text(app: AppHandle, source: String) -> Result<(), String> {
    translate_bilingual_hotkey_flow_core(app, Some(source), false).await;
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
    register_translate_hotkeys(&app, &preset)?;
    #[cfg(target_os = "linux")]
    if let Some(w) = app.try_state::<wayland_shortcut::HotkeyPresetWatch>() {
        let _ = w.0.send(preset.clone());
    }
    let _ = app.emit("translate-hotkey-changed", ());
    Ok(())
}

/// 与全局热键相同逻辑；可从主窗口按钮调用（Wayland 下全局快捷键常不可用）。
#[tauri::command]
async fn trigger_translate_overlay(app: AppHandle) -> Result<(), String> {
    translate_flow_selection_first(app).await;
    Ok(())
}

/// 与 Ctrl+Shift+2 相同：中英翻译浮层（先 PRIMARY 划词，再剪贴板）。
#[tauri::command]
async fn trigger_zh_en_overlay(app: AppHandle) -> Result<(), String> {
    translate_zh_en_selection_first(app).await;
    Ok(())
}

#[tauri::command]
fn write_clipboard_text(text: String) -> Result<(), String> {
    selection::write_clipboard_text(&text)
}

#[tauri::command]
fn get_weekly_article_status(
    vocab: tauri::State<VocabStore>,
    weekly: tauri::State<WeeklyArticleStore>,
) -> Result<WeeklyStatusDto, String> {
    weekly.get_status(&vocab)
}

#[tauri::command]
async fn generate_weekly_article(
    app: AppHandle,
    vocab: tauri::State<'_, VocabStore>,
    weekly: tauri::State<'_, WeeklyArticleStore>,
) -> Result<SavedArticle, String> {
    let api_key = std::env::var("DASHSCOPE_API_KEY").map_err(|_| {
        "未配置 DASHSCOPE_API_KEY。请在启动终端执行 export DASHSCOPE_API_KEY=…（与翻译浮层相同）。".to_string()
    })?;
    let api_key = api_key.trim().to_string();
    if api_key.is_empty() {
        return Err("DASHSCOPE_API_KEY 为空。".to_string());
    }

    let phrases = weekly.take_phrases_for_llm(&vocab)?;
    let numbered = phrases
        .iter()
        .enumerate()
        .map(|(i, p)| format!("{}. {}", i + 1, p))
        .collect::<Vec<_>>()
        .join("\n");

    let raw = translate::weekly_article_completion(&api_key, &numbered).await?;
    let article = weekly.finish_generation(&raw, &phrases)?;
    let _ = app.emit("weekly-article-changed", ());
    Ok(article)
}

#[tauri::command]
fn list_todo_items(store: tauri::State<TodoStore>) -> Result<Vec<TodoItem>, String> {
    store.list_items()
}

#[tauri::command]
fn add_todo_item(
    app: AppHandle,
    store: tauri::State<TodoStore>,
    title: String,
    notes: Option<String>,
    due_at: Option<String>,
) -> Result<TodoItem, String> {
    let item = store.add_item(title, notes, due_at)?;
    let _ = app.emit("todo-changed", ());
    Ok(item)
}

#[tauri::command]
fn update_todo_item(
    app: AppHandle,
    store: tauri::State<TodoStore>,
    id: String,
    title: String,
    notes: String,
    due_at: Option<String>,
) -> Result<TodoItem, String> {
    let item = store.update_item(&id, title, notes, due_at)?;
    let _ = app.emit("todo-changed", ());
    Ok(item)
}

#[tauri::command]
fn set_todo_completed(
    app: AppHandle,
    store: tauri::State<TodoStore>,
    id: String,
    completed: bool,
) -> Result<(), String> {
    store.set_completed(&id, completed)?;
    let _ = app.emit("todo-changed", ());
    Ok(())
}

#[tauri::command]
fn delete_todo_item(
    app: AppHandle,
    store: tauri::State<TodoStore>,
    id: String,
) -> Result<(), String> {
    store.delete_item(&id)?;
    let _ = app.emit("todo-changed", ());
    let _ = app.emit("todo-schedules-changed", ());
    Ok(())
}

#[tauri::command]
fn list_todo_schedules(store: tauri::State<TodoStore>) -> Result<Vec<TodoSchedule>, String> {
    store.list_schedules()
}

#[tauri::command]
fn add_todo_schedule(
    app: AppHandle,
    store: tauri::State<TodoStore>,
    title: String,
    fire_at: String,
    todo_id: Option<String>,
) -> Result<TodoSchedule, String> {
    let sch = store.add_schedule(title, fire_at, todo_id)?;
    let _ = app.emit("todo-schedules-changed", ());
    Ok(sch)
}

#[tauri::command]
fn delete_todo_schedule(
    app: AppHandle,
    store: tauri::State<TodoStore>,
    id: String,
) -> Result<(), String> {
    store.delete_schedule(&id)?;
    let _ = app.emit("todo-schedules-changed", ());
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let global_shortcut_plugin = tauri_plugin_global_shortcut::Builder::new().build();

    tauri::Builder::default()
        .plugin(global_shortcut_plugin)
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let dir = app.path().app_data_dir().map_err(|e| -> Box<dyn std::error::Error> {
                e.to_string().into()
            })?;
            std::fs::create_dir_all(&dir).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let db = storage::open_database(&dir).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            storage::migrate::run_from_json_files(&db, &dir)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let store = VocabStore::open(&db).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let star_migrate_flag = dir.join("migrated_vocab_star_v1.txt");
            if !star_migrate_flag.exists() {
                let _ = store.migrate_legacy_unstarred_to_starred();
                let _ = std::fs::write(&star_migrate_flag, "1");
            }
            app.manage(store);
            let recent_store =
                RecentTranslationsStore::open(&db).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            app.manage(recent_store);
            let weekly_store =
                WeeklyArticleStore::open(&db).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            app.manage(weekly_store);
            let app_settings = AppSettings::open(&db).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let preset = app_settings.preset();
            app.manage(app_settings);
            let todo_store = TodoStore::open(&db).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            app.manage(todo_store);
            todo_notify::spawn_schedule_notification_loop(app.handle().clone());
            #[cfg(target_os = "linux")]
            if std::env::var_os("WAYLAND_DISPLAY").is_some() {
                let (tx, rx) = tokio::sync::watch::channel(preset.clone());
                let app_h = app.handle().clone();
                tauri::async_runtime::spawn(wayland_shortcut::portal_hotkey_loop(app_h, rx));
                app.manage(wayland_shortcut::HotkeyPresetWatch(tx));
                eprintln!(
                    "[WordWing] Wayland：正在通过桌面门户注册全局翻译快捷键；首次使用请在系统对话框中确认。"
                );
            }
            register_translate_hotkeys(app.handle(), &preset)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_vocabulary,
            add_vocabulary_item,
            list_recent_translations_page,
            delete_vocabulary_item,
            set_vocabulary_starred,
            record_vocab_review,
            translate_from_clipboard_only,
            translate_from_clipboard_zh_en,
            retry_translate_with_text,
            retry_translate_zh_en_with_text,
            write_clipboard_text,
            trigger_zh_en_overlay,
            get_translate_hotkey_preset,
            get_translate_hotkey_display,
            list_translate_hotkey_choices,
            set_translate_hotkey_preset,
            trigger_translate_overlay,
            get_weekly_article_status,
            generate_weekly_article,
            list_todo_items,
            add_todo_item,
            update_todo_item,
            set_todo_completed,
            delete_todo_item,
            list_todo_schedules,
            add_todo_schedule,
            delete_todo_schedule
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
