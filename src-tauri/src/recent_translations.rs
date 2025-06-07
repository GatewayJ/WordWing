use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

const MAX_RECENT: usize = 100;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RecentTranslationItem {
    pub id: String,
    pub source_text: String,
    pub translation: String,
    pub target_lang: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct RecentTranslationsPage {
    pub items: Vec<RecentTranslationItem>,
    pub total: usize,
}

pub struct RecentTranslationsStore {
    path: PathBuf,
    items: Mutex<Vec<RecentTranslationItem>>,
}

impl RecentTranslationsStore {
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let items = if path.exists() {
            let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            if raw.trim().is_empty() {
                Vec::new()
            } else {
                serde_json::from_str(&raw).map_err(|e| e.to_string())?
            }
        } else {
            Vec::new()
        };
        Ok(Self {
            path,
            items: Mutex::new(items),
        })
    }

    fn persist_locked(items: &[RecentTranslationItem], path: &PathBuf) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let raw = serde_json::to_string_pretty(items).map_err(|e| e.to_string())?;
        fs::write(path, raw).map_err(|e| e.to_string())
    }

    /// 新记录插到最前，总量不超过 MAX_RECENT。
    pub fn push(
        &self,
        source_text: String,
        translation: String,
        target_lang: String,
    ) -> Result<(), String> {
        let mut g = self.items.lock().map_err(|e| e.to_string())?;
        let item = RecentTranslationItem {
            id: Uuid::new_v4().to_string(),
            source_text: source_text.trim().to_string(),
            translation: translation.trim().to_string(),
            target_lang,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        g.insert(0, item);
        if g.len() > MAX_RECENT {
            g.truncate(MAX_RECENT);
        }
        Self::persist_locked(&g, &self.path)?;
        Ok(())
    }

    /// page 从 1 开始；per_page 限制在 1–50。
    pub fn list_page(&self, page: u32, per_page: u32) -> Result<RecentTranslationsPage, String> {
        let g = self.items.lock().map_err(|e| e.to_string())?;
        let per = (per_page.clamp(1, 50)) as usize;
        let p = page.max(1) as usize;
        let total = g.len();
        let start = (p - 1).saturating_mul(per);
        let items = g.iter().skip(start).take(per).cloned().collect();
        Ok(RecentTranslationsPage { items, total })
    }
}
