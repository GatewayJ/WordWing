use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn default_review_ease_milli() -> u16 {
    2500
}

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
    /// 间隔重复：当前复习间隔（天）。
    #[serde(default)]
    pub review_interval_days: u32,
    /// 间隔重复：难度系数，2500 表示 2.5。
    #[serde(default = "default_review_ease_milli")]
    pub review_ease_milli: u16,
    #[serde(default)]
    pub next_review_at: Option<String>,
    #[serde(default)]
    pub last_reviewed_at: Option<String>,
}

#[derive(Serialize)]
pub struct ReviewQueue {
    pub items: Vec<VocabItem>,
    pub total: usize,
    pub next_due_at: Option<String>,
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
        let source_text = source_text.trim().to_string();
        let translation = translation.trim().to_string();
        let target_lang = target_lang.trim().to_string();
        if source_text.is_empty() || translation.is_empty() || target_lang.is_empty() {
            return Err("原文、译文和目标语言不能为空".to_string());
        }

        // 同一原文和目标语言视为同一词条：更新译文并合并星标，保留复习进度。
        for entry in self.tree.iter() {
            let (key, value) = entry.map_err(|e| e.to_string())?;
            let mut existing: VocabItem =
                serde_json::from_slice(&value).map_err(|e| e.to_string())?;
            if existing.source_text.eq_ignore_ascii_case(&source_text)
                && existing.target_lang.eq_ignore_ascii_case(&target_lang)
            {
                existing.translation = translation;
                existing.starred |= starred;
                let value = serde_json::to_vec(&existing).map_err(|e| e.to_string())?;
                self.tree.insert(key, value).map_err(|e| e.to_string())?;
                self.tree.flush().map_err(|e| e.to_string())?;
                return Ok(existing);
            }
        }

        let item = VocabItem {
            id: Uuid::new_v4().to_string(),
            source_text,
            translation,
            target_lang,
            created_at: chrono::Utc::now().to_rfc3339(),
            starred,
            review_correct: 0,
            review_miss: 0,
            review_interval_days: 0,
            review_ease_milli: default_review_ease_milli(),
            next_review_at: None,
            last_reviewed_at: None,
        };
        let val = serde_json::to_vec(&item).map_err(|e| e.to_string())?;
        self.tree
            .insert(item.id.as_bytes(), val)
            .map_err(|e| e.to_string())?;
        self.tree.flush().map_err(|e| e.to_string())?;
        Ok(item)
    }

    pub fn update(
        &self,
        id: &str,
        source_text: String,
        translation: String,
        target_lang: String,
    ) -> Result<VocabItem, String> {
        let source_text = source_text.trim().to_string();
        let translation = translation.trim().to_string();
        let target_lang = target_lang.trim().to_string();
        if source_text.is_empty() || translation.is_empty() || target_lang.is_empty() {
            return Err("原文、译文和目标语言不能为空".to_string());
        }
        for entry in self.tree.iter() {
            let (_, value) = entry.map_err(|e| e.to_string())?;
            let other: VocabItem = serde_json::from_slice(&value).map_err(|e| e.to_string())?;
            if other.id != id
                && other.source_text.eq_ignore_ascii_case(&source_text)
                && other.target_lang.eq_ignore_ascii_case(&target_lang)
            {
                return Err("已存在相同原文和目标语言的词条".to_string());
            }
        }
        let old = self
            .tree
            .get(id.as_bytes())
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未找到该条目".to_string())?;
        let mut item: VocabItem = serde_json::from_slice(&old).map_err(|e| e.to_string())?;
        item.source_text = source_text;
        item.translation = translation;
        item.target_lang = target_lang;
        let value = serde_json::to_vec(&item).map_err(|e| e.to_string())?;
        self.tree
            .insert(id.as_bytes(), value)
            .map_err(|e| e.to_string())?;
        self.tree.flush().map_err(|e| e.to_string())?;
        Ok(item)
    }

    pub fn review_queue(&self) -> Result<ReviewQueue, String> {
        let all = self.list()?;
        let total = all.len();
        let now = chrono::Utc::now();
        let mut next_due: Option<chrono::DateTime<chrono::Utc>> = None;
        let mut items = Vec::new();
        for item in all {
            let due = item
                .next_review_at
                .as_deref()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&chrono::Utc));
            match due {
                None => items.push(item),
                Some(value) if value <= now => items.push(item),
                Some(value) => {
                    if next_due.is_none_or(|current| value < current) {
                        next_due = Some(value);
                    }
                }
            }
        }
        items.sort_by(|a, b| {
            b.review_miss
                .cmp(&a.review_miss)
                .then_with(|| a.next_review_at.cmp(&b.next_review_at))
        });
        Ok(ReviewQueue {
            items,
            total,
            next_due_at: next_due.map(|value| value.to_rfc3339()),
        })
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
        let now = chrono::Utc::now();
        let ease = if item.review_ease_milli == 0 {
            default_review_ease_milli()
        } else {
            item.review_ease_milli
        };
        if remembered {
            item.review_correct = item.review_correct.saturating_add(1);
            item.review_interval_days = match item.review_interval_days {
                0 => 1,
                1 => 3,
                days => ((days as u64 * ease as u64 / 1000) as u32)
                    .max(days.saturating_add(1))
                    .min(365),
            };
            item.review_ease_milli = ease.saturating_add(100).min(3000);
            item.next_review_at =
                Some((now + chrono::Duration::days(item.review_interval_days as i64)).to_rfc3339());
        } else {
            item.review_miss = item.review_miss.saturating_add(1);
            item.review_interval_days = 0;
            item.review_ease_milli = ease.saturating_sub(200).max(1300);
            item.next_review_at = Some((now + chrono::Duration::minutes(10)).to_rfc3339());
        }
        item.last_reviewed_at = Some(now.to_rfc3339());
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

#[cfg(test)]
mod tests {
    use super::VocabStore;

    fn store() -> VocabStore {
        let db = sled::Config::new().temporary(true).open().unwrap();
        VocabStore::open(&db).unwrap()
    }

    #[test]
    fn add_deduplicates_and_preserves_progress() {
        let store = store();
        let first = store
            .add("Hello".into(), "你好".into(), "中文".into(), false)
            .unwrap();
        store.record_review(&first.id, true).unwrap();
        let second = store
            .add("hello".into(), "您好".into(), "中文".into(), true)
            .unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(second.translation, "您好");
        assert!(second.starred);
        assert_eq!(second.review_correct, 1);
        assert_eq!(store.list().unwrap().len(), 1);
    }

    #[test]
    fn review_moves_item_out_of_due_queue() {
        let store = store();
        let item = store
            .add("world".into(), "世界".into(), "中文".into(), true)
            .unwrap();
        assert_eq!(store.review_queue().unwrap().items.len(), 1);
        store.record_review(&item.id, true).unwrap();
        let queue = store.review_queue().unwrap();
        assert!(queue.items.is_empty());
        assert!(queue.next_due_at.is_some());
    }
}
