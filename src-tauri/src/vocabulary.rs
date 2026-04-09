use serde::{Deserialize, Serialize};
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
    tree: sled::Tree,
}

impl VocabStore {
    pub fn open(db: &sled::Db) -> Result<Self, String> {
        let tree = db.open_tree("vocab").map_err(|e| e.to_string())?;
        Ok(Self { tree })
    }

    pub fn list(&self) -> Result<Vec<VocabItem>, String> {
        let mut v = Vec::new();
        for item in self.tree.iter() {
            let (_, val) = item.map_err(|e| e.to_string())?;
            let it: VocabItem = serde_json::from_slice(&val).map_err(|e| e.to_string())?;
            v.push(it);
        }
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
        let val = serde_json::to_vec(&item).map_err(|e| e.to_string())?;
        self.tree
            .insert(item.id.as_bytes(), val)
            .map_err(|e| e.to_string())?;
        self.tree.flush().map_err(|e| e.to_string())?;
        Ok(item)
    }

    pub fn remove(&self, id: &str) -> Result<(), String> {
        let r = self.tree.remove(id.as_bytes()).map_err(|e| e.to_string())?;
        if r.is_none() {
            return Err("未找到该条目".to_string());
        }
        self.tree.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_starred(&self, id: &str, starred: bool) -> Result<(), String> {
        let old = self
            .tree
            .get(id.as_bytes())
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未找到该条目".to_string())?;
        let mut item: VocabItem = serde_json::from_slice(&old).map_err(|e| e.to_string())?;
        item.starred = starred;
        let val = serde_json::to_vec(&item).map_err(|e| e.to_string())?;
        self.tree
            .insert(id.as_bytes(), val)
            .map_err(|e| e.to_string())?;
        self.tree.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn record_review(&self, id: &str, remembered: bool) -> Result<(), String> {
        let old = self
            .tree
            .get(id.as_bytes())
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未找到该条目".to_string())?;
        let mut item: VocabItem = serde_json::from_slice(&old).map_err(|e| e.to_string())?;
        if remembered {
            item.review_correct = item.review_correct.saturating_add(1);
        } else {
            item.review_miss = item.review_miss.saturating_add(1);
        }
        let val = serde_json::to_vec(&item).map_err(|e| e.to_string())?;
        self.tree
            .insert(id.as_bytes(), val)
            .map_err(|e| e.to_string())?;
        self.tree.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 升级后一次性执行：旧版「加入生词本」无星标字段，全部视为收藏以便出现在收藏页。
    pub fn migrate_legacy_unstarred_to_starred(&self) -> Result<(), String> {
        let mut changed = false;
        for entry in self.tree.iter() {
            let (k, v) = entry.map_err(|e| e.to_string())?;
            let mut item: VocabItem = serde_json::from_slice(&v).map_err(|e| e.to_string())?;
            if !item.starred {
                item.starred = true;
                changed = true;
                let val = serde_json::to_vec(&item).map_err(|e| e.to_string())?;
                self.tree.insert(k, val).map_err(|e| e.to_string())?;
            }
        }
        if changed {
            self.tree.flush().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
