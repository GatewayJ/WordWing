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
    ) -> Result<VocabItem, String> {
        let mut g = self.items.lock().map_err(|e| e.to_string())?;
        let item = VocabItem {
            id: Uuid::new_v4().to_string(),
            source_text: source_text.trim().to_string(),
            translation: translation.trim().to_string(),
            target_lang,
            created_at: chrono::Utc::now().to_rfc3339(),
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
}
