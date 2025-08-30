//! Todo 条目与定时提醒，持久化于 sled 树 `todo_items` / `todo_schedules`。

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TodoItem {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub notes: String,
    pub due_at: Option<String>,
    #[serde(default)]
    pub completed: bool,
    pub created_at: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TodoSchedule {
    pub id: String,
    /// 关联条目；`None` 为独立提醒
    pub todo_id: Option<String>,
    pub title: String,
    pub fire_at: String,
    pub created_at: String,
    /// 是否已发送系统通知（到期只推一次）
    #[serde(default)]
    pub notification_sent: bool,
}

pub struct TodoStore {
    items: sled::Tree,
    schedules: sled::Tree,
}

impl TodoStore {
    pub fn open(db: &sled::Db) -> Result<Self, String> {
        let s = Self {
            items: db.open_tree("todo_items").map_err(|e| e.to_string())?,
            schedules: db
                .open_tree("todo_schedules")
                .map_err(|e| e.to_string())?,
        };
        s.migrate_past_schedules_skip_notification()?;
        Ok(s)
    }

    /// 升级后一次性处理：已过触发时间且尚未标记的记录，视为无需再推送，避免首次轮询刷屏。
    fn migrate_past_schedules_skip_notification(&self) -> Result<(), String> {
        let now = chrono::Utc::now();
        for entry in self.schedules.iter() {
            let (k, v) = entry.map_err(|e| e.to_string())?;
            let mut sch: TodoSchedule = serde_json::from_slice(&v).map_err(|e| e.to_string())?;
            if sch.notification_sent {
                continue;
            }
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&sch.fire_at) {
                let t = dt.with_timezone(&chrono::Utc);
                if t < now {
                    sch.notification_sent = true;
                    let val = serde_json::to_vec(&sch).map_err(|e| e.to_string())?;
                    self.schedules.insert(k, val).map_err(|e| e.to_string())?;
                }
            }
        }
        self.schedules.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_items(&self) -> Result<Vec<TodoItem>, String> {
        let mut v = Vec::new();
        for entry in self.items.iter() {
            let (_, val) = entry.map_err(|e| e.to_string())?;
            let it: TodoItem = serde_json::from_slice(&val).map_err(|e| e.to_string())?;
            v.push(it);
        }
        v.sort_by(|a, b| {
            match (a.completed, b.completed) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => {
                    let due_ord = match (&a.due_at, &b.due_at) {
                        (None, None) => std::cmp::Ordering::Equal,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (Some(x), Some(y)) => x.cmp(y),
                    };
                    if due_ord != std::cmp::Ordering::Equal {
                        return due_ord;
                    }
                    b.created_at.cmp(&a.created_at)
                }
            }
        });
        Ok(v)
    }

    pub fn add_item(
        &self,
        title: String,
        notes: Option<String>,
        due_at: Option<String>,
    ) -> Result<TodoItem, String> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err("标题不能为空".to_string());
        }
        if let Some(ref d) = due_at {
            validate_rfc3339(d)?;
        }
        let item = TodoItem {
            id: Uuid::new_v4().to_string(),
            title,
            notes: notes.unwrap_or_default().trim().to_string(),
            due_at,
            completed: false,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        let val = serde_json::to_vec(&item).map_err(|e| e.to_string())?;
        self.items
            .insert(item.id.as_bytes(), val)
            .map_err(|e| e.to_string())?;
        self.items.flush().map_err(|e| e.to_string())?;
        Ok(item)
    }

    pub fn update_item(
        &self,
        id: &str,
        title: String,
        notes: String,
        due_at: Option<String>,
    ) -> Result<TodoItem, String> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err("标题不能为空".to_string());
        }
        if let Some(ref d) = due_at {
            validate_rfc3339(d)?;
        }
        let old = self
            .items
            .get(id.as_bytes())
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未找到该条目".to_string())?;
        let mut item: TodoItem = serde_json::from_slice(&old).map_err(|e| e.to_string())?;
        item.title = title;
        item.notes = notes.trim().to_string();
        item.due_at = due_at;
        let val = serde_json::to_vec(&item).map_err(|e| e.to_string())?;
        self.items
            .insert(id.as_bytes(), val)
            .map_err(|e| e.to_string())?;
        self.items.flush().map_err(|e| e.to_string())?;
        Ok(item)
    }

    pub fn set_completed(&self, id: &str, completed: bool) -> Result<(), String> {
        let old = self
            .items
            .get(id.as_bytes())
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未找到该条目".to_string())?;
        let mut item: TodoItem = serde_json::from_slice(&old).map_err(|e| e.to_string())?;
        item.completed = completed;
        let val = serde_json::to_vec(&item).map_err(|e| e.to_string())?;
        self.items
            .insert(id.as_bytes(), val)
            .map_err(|e| e.to_string())?;
        self.items.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_item(&self, id: &str) -> Result<(), String> {
        let r = self.items.remove(id.as_bytes()).map_err(|e| e.to_string())?;
        if r.is_none() {
            return Err("未找到该条目".to_string());
        }
        let mut drop_keys: Vec<Vec<u8>> = Vec::new();
        for entry in self.schedules.iter() {
            let (k, v) = entry.map_err(|e| e.to_string())?;
            let s: TodoSchedule = serde_json::from_slice(&v).map_err(|e| e.to_string())?;
            if s.todo_id.as_deref() == Some(id) {
                drop_keys.push(k.to_vec());
            }
        }
        for k in drop_keys {
            self.schedules.remove(k).map_err(|e| e.to_string())?;
        }
        self.items.flush().map_err(|e| e.to_string())?;
        self.schedules.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_schedules(&self) -> Result<Vec<TodoSchedule>, String> {
        let mut v = Vec::new();
        for entry in self.schedules.iter() {
            let (_, val) = entry.map_err(|e| e.to_string())?;
            let s: TodoSchedule = serde_json::from_slice(&val).map_err(|e| e.to_string())?;
            v.push(s);
        }
        v.sort_by(|a, b| a.fire_at.cmp(&b.fire_at));
        Ok(v)
    }

    pub fn add_schedule(
        &self,
        title: String,
        fire_at: String,
        todo_id: Option<String>,
    ) -> Result<TodoSchedule, String> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err("标题不能为空".to_string());
        }
        validate_rfc3339(&fire_at)?;
        if let Some(ref tid) = todo_id {
            if self.items.get(tid.as_bytes()).map_err(|e| e.to_string())?.is_none() {
                return Err("关联的待办条目不存在".to_string());
            }
        }
        let sch = TodoSchedule {
            id: Uuid::new_v4().to_string(),
            todo_id,
            title,
            fire_at,
            created_at: chrono::Utc::now().to_rfc3339(),
            notification_sent: false,
        };
        let val = serde_json::to_vec(&sch).map_err(|e| e.to_string())?;
        self.schedules
            .insert(sch.id.as_bytes(), val)
            .map_err(|e| e.to_string())?;
        self.schedules.flush().map_err(|e| e.to_string())?;
        Ok(sch)
    }

    pub fn delete_schedule(&self, id: &str) -> Result<(), String> {
        let r = self
            .schedules
            .remove(id.as_bytes())
            .map_err(|e| e.to_string())?;
        if r.is_none() {
            return Err("未找到该定时".to_string());
        }
        self.schedules.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 已到触发时间、尚未推送系统通知的定时（`fire_at <= now`）。
    pub fn list_due_unsent(&self, now: &chrono::DateTime<chrono::Utc>) -> Result<Vec<TodoSchedule>, String> {
        let mut out = Vec::new();
        for entry in self.schedules.iter() {
            let (_, v) = entry.map_err(|e| e.to_string())?;
            let s: TodoSchedule = serde_json::from_slice(&v).map_err(|e| e.to_string())?;
            if s.notification_sent {
                continue;
            }
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s.fire_at) {
                let t = dt.with_timezone(&chrono::Utc);
                if t <= *now {
                    out.push(s);
                }
            }
        }
        Ok(out)
    }

    pub fn mark_schedule_notification_sent(&self, id: &str) -> Result<(), String> {
        let old = self
            .schedules
            .get(id.as_bytes())
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未找到该定时".to_string())?;
        let mut s: TodoSchedule = serde_json::from_slice(&old).map_err(|e| e.to_string())?;
        s.notification_sent = true;
        let val = serde_json::to_vec(&s).map_err(|e| e.to_string())?;
        self.schedules
            .insert(id.as_bytes(), val)
            .map_err(|e| e.to_string())?;
        self.schedules.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn validate_rfc3339(s: &str) -> Result<(), String> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map_err(|_| "时间格式无效，请使用有效的日期时间".to_string())?;
    Ok(())
}
