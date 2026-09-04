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
    /// 关联条目完成时自动暂停；重新标记未完成会恢复。
    #[serde(default)]
    pub cancelled: bool,
}

pub struct TodoStore {
    items: sled::Tree,
    schedules: sled::Tree,
}

impl TodoStore {
    pub fn open(db: &sled::Db) -> Result<Self, String> {
        Ok(Self {
            items: db.open_tree("todo_items").map_err(|e| e.to_string())?,
            schedules: db.open_tree("todo_schedules").map_err(|e| e.to_string())?,
        })
    }

    pub fn list_items(&self) -> Result<Vec<TodoItem>, String> {
        let mut v = Vec::new();
        for entry in self.items.iter() {
            let (_, val) = entry.map_err(|e| e.to_string())?;
            let it: TodoItem = serde_json::from_slice(&val).map_err(|e| e.to_string())?;
            v.push(it);
        }
        v.sort_by(|a, b| match (a.completed, b.completed) {
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
        for entry in self.schedules.iter() {
            let (key, value) = entry.map_err(|e| e.to_string())?;
            let mut schedule: TodoSchedule =
                serde_json::from_slice(&value).map_err(|e| e.to_string())?;
            if schedule.todo_id.as_deref() == Some(id) && !schedule.notification_sent {
                schedule.cancelled = completed;
                let value = serde_json::to_vec(&schedule).map_err(|e| e.to_string())?;
                self.schedules
                    .insert(key, value)
                    .map_err(|e| e.to_string())?;
            }
        }
        self.schedules.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_item(&self, id: &str) -> Result<(), String> {
        let r = self
            .items
            .remove(id.as_bytes())
            .map_err(|e| e.to_string())?;
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
        let mut linked_todo_completed = false;
        if let Some(ref tid) = todo_id {
            let item = self
                .items
                .get(tid.as_bytes())
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "关联的待办条目不存在".to_string())?;
            let item: TodoItem = serde_json::from_slice(&item).map_err(|e| e.to_string())?;
            linked_todo_completed = item.completed;
        }
        let sch = TodoSchedule {
            id: Uuid::new_v4().to_string(),
            todo_id,
            title,
            fire_at,
            created_at: chrono::Utc::now().to_rfc3339(),
            notification_sent: false,
            cancelled: linked_todo_completed,
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

    pub fn update_schedule(
        &self,
        id: &str,
        title: String,
        fire_at: String,
        todo_id: Option<String>,
    ) -> Result<TodoSchedule, String> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err("标题不能为空".to_string());
        }
        validate_rfc3339(&fire_at)?;
        let mut linked_todo_completed = false;
        if let Some(ref todo_id) = todo_id {
            let item = self
                .items
                .get(todo_id.as_bytes())
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "关联的待办条目不存在".to_string())?;
            let item: TodoItem = serde_json::from_slice(&item).map_err(|e| e.to_string())?;
            linked_todo_completed = item.completed;
        }
        let old = self
            .schedules
            .get(id.as_bytes())
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未找到该定时".to_string())?;
        let mut schedule: TodoSchedule = serde_json::from_slice(&old).map_err(|e| e.to_string())?;
        schedule.title = title;
        schedule.fire_at = fire_at;
        schedule.todo_id = todo_id;
        // 修改提醒等同于重新安排，即使旧提醒已经推送也应按新时间再次提醒。
        schedule.notification_sent = false;
        schedule.cancelled = linked_todo_completed;
        let value = serde_json::to_vec(&schedule).map_err(|e| e.to_string())?;
        self.schedules
            .insert(id.as_bytes(), value)
            .map_err(|e| e.to_string())?;
        self.schedules.flush().map_err(|e| e.to_string())?;
        Ok(schedule)
    }

    /// 已到触发时间、尚未推送系统通知的定时（`fire_at <= now`）。
    pub fn list_due_unsent(
        &self,
        now: &chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<TodoSchedule>, String> {
        let mut out = Vec::new();
        for entry in self.schedules.iter() {
            let (_, v) = entry.map_err(|e| e.to_string())?;
            let s: TodoSchedule = serde_json::from_slice(&v).map_err(|e| e.to_string())?;
            if s.notification_sent || s.cancelled {
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

#[cfg(test)]
mod tests {
    use super::TodoStore;

    #[test]
    fn overdue_unsent_schedule_remains_due_for_catch_up() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = TodoStore::open(&db).unwrap();
        let schedule = store
            .add_schedule("catch up".into(), "2020-01-01T00:00:00Z".into(), None)
            .unwrap();
        let due = store.list_due_unsent(&chrono::Utc::now()).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, schedule.id);
    }

    #[test]
    fn editing_sent_schedule_rearms_it() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = TodoStore::open(&db).unwrap();
        let schedule = store
            .add_schedule("first".into(), "2020-01-01T00:00:00Z".into(), None)
            .unwrap();
        store.mark_schedule_notification_sent(&schedule.id).unwrap();
        let updated = store
            .update_schedule(
                &schedule.id,
                "again".into(),
                "2020-01-02T00:00:00Z".into(),
                None,
            )
            .unwrap();
        assert!(!updated.notification_sent);
        assert_eq!(updated.title, "again");
    }

    #[test]
    fn completing_todo_pauses_and_reopening_restores_reminder() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = TodoStore::open(&db).unwrap();
        let item = store.add_item("todo".into(), None, None).unwrap();
        let schedule = store
            .add_schedule(
                "linked".into(),
                "2020-01-01T00:00:00Z".into(),
                Some(item.id.clone()),
            )
            .unwrap();
        store.set_completed(&item.id, true).unwrap();
        assert!(store
            .list_due_unsent(&chrono::Utc::now())
            .unwrap()
            .is_empty());
        store.set_completed(&item.id, false).unwrap();
        let due = store.list_due_unsent(&chrono::Utc::now()).unwrap();
        assert_eq!(due[0].id, schedule.id);
    }
}
