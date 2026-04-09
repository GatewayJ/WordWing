//! 自旧版 JSON 文件迁移至 sled（幂等：仅在目标树为空且存在旧文件时导入）。

use std::fs;
use std::path::Path;

use crate::recent_translations::RecentTranslationItem;
use crate::vocabulary::VocabItem;
use crate::weekly_article::WeeklyFileState;

const VOCAB_JSON: &str = "vocabulary.json";
const RECENT_JSON: &str = "recent_translations.json";
const WEEKLY_JSON: &str = "weekly_generated.json";
const SETTINGS_JSON: &str = "app_settings.json";

pub fn run_from_json_files(db: &sled::Db, app_data_dir: &Path) -> Result<(), String> {
    migrate_vocab(db, app_data_dir)?;
    migrate_recent(db, app_data_dir)?;
    migrate_weekly(db, app_data_dir)?;
    migrate_settings(db, app_data_dir)?;
    Ok(())
}

fn migrate_vocab(db: &sled::Db, dir: &Path) -> Result<(), String> {
    let tree = db.open_tree("vocab").map_err(|e| e.to_string())?;
    if tree.iter().next().is_some() {
        return Ok(());
    }
    let path = dir.join(VOCAB_JSON);
    if !path.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if raw.trim().is_empty() {
        backup_rename(&path)?;
        return Ok(());
    }
    let items: Vec<VocabItem> = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    for it in &items {
        let val = serde_json::to_vec(it).map_err(|e| e.to_string())?;
        tree.insert(it.id.as_bytes(), val)
            .map_err(|e| e.to_string())?;
    }
    tree.flush().map_err(|e| e.to_string())?;
    backup_rename(&path)?;
    Ok(())
}

fn migrate_recent(db: &sled::Db, dir: &Path) -> Result<(), String> {
    let tree = db.open_tree("recent").map_err(|e| e.to_string())?;
    if tree.get(b"!order").map_err(|e| e.to_string())?.is_some() {
        return Ok(());
    }
    let path = dir.join(RECENT_JSON);
    if !path.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if raw.trim().is_empty() {
        backup_rename(&path)?;
        return Ok(());
    }
    let items: Vec<RecentTranslationItem> =
        serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let order: Vec<String> = items.iter().map(|x| x.id.clone()).collect();
    for it in &items {
        let val = serde_json::to_vec(it).map_err(|e| e.to_string())?;
        tree.insert(it.id.as_bytes(), val)
            .map_err(|e| e.to_string())?;
    }
    let order_raw = serde_json::to_vec(&order).map_err(|e| e.to_string())?;
    tree.insert(b"!order", order_raw)
        .map_err(|e| e.to_string())?;
    tree.flush().map_err(|e| e.to_string())?;
    backup_rename(&path)?;
    Ok(())
}

fn migrate_weekly(db: &sled::Db, dir: &Path) -> Result<(), String> {
    let tree = db.open_tree("weekly").map_err(|e| e.to_string())?;
    if tree.get(b"state").map_err(|e| e.to_string())?.is_some() {
        return Ok(());
    }
    let path = dir.join(WEEKLY_JSON);
    if !path.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if raw.trim().is_empty() {
        backup_rename(&path)?;
        return Ok(());
    }
    let state: WeeklyFileState = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let val = serde_json::to_vec(&state).map_err(|e| e.to_string())?;
    tree.insert(b"state", val).map_err(|e| e.to_string())?;
    tree.flush().map_err(|e| e.to_string())?;
    backup_rename(&path)?;
    Ok(())
}

fn migrate_settings(db: &sled::Db, dir: &Path) -> Result<(), String> {
    let tree = db.open_tree("settings").map_err(|e| e.to_string())?;
    if tree.get(b"app").map_err(|e| e.to_string())?.is_some() {
        return Ok(());
    }
    let path = dir.join(SETTINGS_JSON);
    if !path.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if raw.trim().is_empty() {
        backup_rename(&path)?;
        return Ok(());
    }
    tree.insert(b"app", raw.as_bytes().to_vec())
        .map_err(|e| e.to_string())?;
    tree.flush().map_err(|e| e.to_string())?;
    backup_rename(&path)?;
    Ok(())
}

fn backup_rename(path: &Path) -> Result<(), String> {
    let bak = path.with_extension("json.bak");
    let _ = fs::rename(path, &bak);
    Ok(())
}
