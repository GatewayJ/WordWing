use serde::{Deserialize, Serialize};
use uuid::Uuid;

const MAX_RECENT: usize = 100;
const ORDER_KEY: &[u8] = b"!order";

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
    tree: sled::Tree,
}

impl RecentTranslationsStore {
    pub fn open(db: &sled::Db) -> Result<Self, String> {
        let tree = db.open_tree("recent").map_err(|e| e.to_string())?;
        Ok(Self { tree })
    }

    fn load_order(&self) -> Result<Vec<String>, String> {
        match self.tree.get(ORDER_KEY).map_err(|e| e.to_string())? {
            None => Ok(Vec::new()),
            Some(v) => serde_json::from_slice(&v).map_err(|e| e.to_string()),
        }
    }

    fn save_order(&self, order: &[String]) -> Result<(), String> {
        let raw = serde_json::to_vec(order).map_err(|e| e.to_string())?;
        self.tree
            .insert(ORDER_KEY, raw)
            .map_err(|e| e.to_string())?;
        self.tree.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 新记录插到最前，总量不超过 MAX_RECENT。
    pub fn push(
        &self,
        source_text: String,
        translation: String,
        target_lang: String,
    ) -> Result<(), String> {
        let item = RecentTranslationItem {
            id: Uuid::new_v4().to_string(),
            source_text: source_text.trim().to_string(),
            translation: translation.trim().to_string(),
            target_lang,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        let val = serde_json::to_vec(&item).map_err(|e| e.to_string())?;
        self.tree
            .insert(item.id.as_bytes(), val)
            .map_err(|e| e.to_string())?;

        let mut order = self.load_order()?;
        order.insert(0, item.id.clone());
        if order.len() > MAX_RECENT {
            let dropped: Vec<String> = order.drain(MAX_RECENT..).collect();
            for id in dropped {
                let _ = self.tree.remove(id.as_bytes());
            }
        }
        self.save_order(&order)?;
        Ok(())
    }

    /// page 从 1 开始；per_page 限制在 1–50。
    pub fn list_page(&self, page: u32, per_page: u32) -> Result<RecentTranslationsPage, String> {
        let order = self.load_order()?;
        let total = order.len();
        let per = (per_page.clamp(1, 50)) as usize;
        let p = page.max(1) as usize;
        let start = (p - 1).saturating_mul(per);
        let mut items = Vec::new();
        for id in order.iter().skip(start).take(per) {
            if let Some(v) = self.tree.get(id.as_bytes()).map_err(|e| e.to_string())? {
                let it: RecentTranslationItem =
                    serde_json::from_slice(&v).map_err(|e| e.to_string())?;
                items.push(it);
            }
        }
        Ok(RecentTranslationsPage { items, total })
    }
}
