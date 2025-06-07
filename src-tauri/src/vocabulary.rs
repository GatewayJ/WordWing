use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VocabItem {
    pub id: String,
    pub source_text: String,
    pub translation: String,
    pub target_lang: String,
    pub created_at: String,
    /// 收藏星标（复习页可优先抽中）
    #[serde(default)]
    pub starred: bool,
    /// 复习：答对累计
    #[serde(default)]
    pub review_correct: u32,
    /// 复习：答错累计
    #[serde(default)]
    pub review_miss: u32,
}

pub struct VocabStore {
    path: PathBuf,
    items: Mutex<Vec<VocabItem>>,
}

impl VocabStore {
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

    fn persist_locked(items: &[VocabItem], path: &PathBuf) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let raw = serde_json::to_string_pretty(items).map_err(|e| e.to_string())?;
        fs::write(path, raw).map_err(|e| e.to_string())
    }

    pub fn list(&self) -> Result<Vec<VocabItem>, String> {
        let g = self.items.lock().map_err(|e| e.to_string())?;
        let mut v = g.clone();
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(v)
    }

    pub fn add(
        &self,
        source_text: String,
        translation: String,
        target_lang: String,
        starred: bool,
    ) -> Result<VocabItem, String> {
        let mut g = self.items.lock().map_err(|e| e.to_string())?;
        let item = VocabItem {
            id: Uuid::new_v4().to_string(),
            source_text: source_text.trim().to_string(),
            translation: translation.trim().to_string(),
            target_lang,
            created_at: chrono::Utc::now().to_rfc3339(),
            starred,
            review_correct: 0,
            review_miss: 0,
        };
        g.push(item.clone());
        Self::persist_locked(&g, &self.path)?;
        Ok(item)
    }

    pub fn remove(&self, id: &str) -> Result<(), String> {
        let mut g = self.items.lock().map_err(|e| e.to_string())?;
        let before = g.len();
        g.retain(|x| x.id != id);
        if g.len() == before {
            return Err("未找到该条目".to_string());
        }
        Self::persist_locked(&g, &self.path)?;
        Ok(())
    }

    pub fn set_starred(&self, id: &str, starred: bool) -> Result<(), String> {
        let mut g = self.items.lock().map_err(|e| e.to_string())?;
        let item = g
            .iter_mut()
            .find(|x| x.id == id)
            .ok_or_else(|| "未找到该条目".to_string())?;
        item.starred = starred;
        Self::persist_locked(&g, &self.path)?;
        Ok(())
    }

    pub fn record_review(&self, id: &str, remembered: bool) -> Result<(), String> {
        let mut g = self.items.lock().map_err(|e| e.to_string())?;
        let item = g
            .iter_mut()
            .find(|x| x.id == id)
            .ok_or_else(|| "未找到该条目".to_string())?;
        if remembered {
            item.review_correct = item.review_correct.saturating_add(1);
        } else {
            item.review_miss = item.review_miss.saturating_add(1);
        }
        Self::persist_locked(&g, &self.path)?;
        Ok(())
    }

    /// 升级后一次性执行：旧版「加入生词本」无星标字段，全部视为收藏以便出现在收藏页。
    pub fn migrate_legacy_unstarred_to_starred(&self) -> Result<(), String> {
        let mut g = self.items.lock().map_err(|e| e.to_string())?;
        let mut changed = false;
        for item in g.iter_mut() {
            if !item.starred {
                item.starred = true;
                changed = true;
            }
        }
        if changed {
            Self::persist_locked(&g, &self.path)?;
        }
        Ok(())
    }
}
