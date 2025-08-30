import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Bell } from "lucide-react";
import { PlainDateTimeFields } from "../../components/PlainDateTimeFields";
import {
  dateTimePartsToIso,
  isoToDateTimeParts,
  validateDatePart,
  validateTimePart,
} from "../../lib/datetimeInput";

type TodoItem = {
  id: string;
  title: string;
  notes: string;
  dueAt: string | null;
  completed: boolean;
  createdAt: string;
};

function formatDue(iso: string | null) {
  if (!iso) return "—";
  try {
    return new Date(iso).toLocaleString("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

function isOverdue(item: TodoItem): boolean {
  if (item.completed || !item.dueAt) return false;
  return new Date(item.dueAt).getTime() < Date.now();
}

export function ItemsPage() {
  const [items, setItems] = useState<TodoItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [notes, setNotes] = useState("");
  const [dueDateStr, setDueDateStr] = useState("");
  const [dueTimeStr, setDueTimeStr] = useState("");
  const [saving, setSaving] = useState(false);
  const [editing, setEditing] = useState<TodoItem | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      const list = await invoke<TodoItem[]>("list_todo_items");
      setItems(list);
    } catch (e) {
      setErr(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    let un: UnlistenFn | undefined;
    void listen("todo-changed", () => {
      void load();
    }).then((fn) => {
      un = fn;
    });
    return () => {
      void un?.();
    };
  }, [load]);

  const resetForm = () => {
    setTitle("");
    setNotes("");
    setDueDateStr("");
    setDueTimeStr("");
    setEditing(null);
  };

  const startEdit = (item: TodoItem) => {
    setEditing(item);
    setTitle(item.title);
    setNotes(item.notes ?? "");
    const parts = isoToDateTimeParts(item.dueAt);
    setDueDateStr(parts.date);
    setDueTimeStr(parts.time);
  };

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    const t = title.trim();
    if (!t) {
      setErr("请填写标题");
      return;
    }
    setSaving(true);
    setErr(null);
    let dueAt: string | null = null;
    const dTrim = dueDateStr.trim();
    if (dTrim) {
      if (!validateDatePart(dTrim) || !validateTimePart(dueTimeStr)) {
        setErr("截止时间格式有误：日期为 YYYY-MM-DD，时间为 HH:MM（24 小时）。");
        setSaving(false);
        return;
      }
      const iso = dateTimePartsToIso(dueDateStr, dueTimeStr);
      if (!iso) {
        setErr("截止时间无效，请检查日期是否存在（如闰年、每月天数）。");
        setSaving(false);
        return;
      }
      dueAt = iso;
    }
    try {
      if (editing) {
        await invoke("update_todo_item", {
          id: editing.id,
          title: t,
          notes: notes.trim(),
          dueAt,
        });
      } else {
        await invoke("add_todo_item", {
          title: t,
          notes: notes.trim() || null,
          dueAt,
        });
      }
      resetForm();
      await load();
    } catch (e) {
      setErr(String(e));
    } finally {
      setSaving(false);
    }
  };

  const toggleDone = async (item: TodoItem) => {
    try {
      await invoke("set_todo_completed", {
        id: item.id,
        completed: !item.completed,
      });
      await load();
    } catch (e) {
      setErr(String(e));
    }
  };

  const remove = async (id: string) => {
    if (!window.confirm("确定删除该条目？关联的定时提醒也会一并删除。")) return;
    try {
      await invoke("delete_todo_item", { id });
      await load();
      if (editing?.id === id) resetForm();
    } catch (e) {
      setErr(String(e));
    }
  };

  return (
    <>
      <h2 className="page-title">条目</h2>
      <p className="page-lead">
        任务列表；未完成的逾期项使用左侧 <strong style={{ color: "var(--error)" }}>3px</strong> 强调条（与视觉稿一致）。
        需要提醒时可点行内 <strong>定时</strong> 前往{" "}
        <Link to="/todo/schedules">定时</Link> 页并关联本条目。
      </p>

      {err && (
        <p className="page-lead" style={{ color: "var(--error)" }}>
          {err}
        </p>
      )}

      <div className="card todo-form-card">
        <h3 className="todo-form-card__title">{editing ? "编辑条目" : "新建条目"}</h3>
        <form className="todo-form" onSubmit={(e) => void submit(e)}>
          <label className="settings-label" htmlFor="todo-title">
            标题
          </label>
          <input
            id="todo-title"
            className="todo-input"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="必填"
            autoComplete="off"
          />
          <label className="settings-label" htmlFor="todo-notes">
            备注
          </label>
          <textarea
            id="todo-notes"
            className="todo-textarea"
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            rows={3}
            placeholder="可选"
          />
          <p className="settings-label" style={{ marginBottom: 4 }}>
            截止时间（可选）
          </p>
          <PlainDateTimeFields
            idPrefix="todo-item"
            dateStr={dueDateStr}
            timeStr={dueTimeStr}
            onDateStrChange={setDueDateStr}
            onTimeStrChange={setDueTimeStr}
            dateLabel="日期"
            timeLabel="时间"
            optionalHint="不填表示无截止时间。使用文本输入，避免系统日历弹层与桌面窗口冲突。"
          />
          <div className="todo-form__actions">
            <button type="submit" className="settings-save-btn" disabled={saving}>
              {saving ? "保存中…" : editing ? "保存修改" : "添加"}
            </button>
            {editing && (
              <button type="button" className="btn-secondary" onClick={resetForm}>
                取消编辑
              </button>
            )}
          </div>
        </form>
      </div>

      {loading ? (
        <p className="page-lead">加载中…</p>
      ) : items.length === 0 ? (
        <div className="card">
          <p>暂无条目。在上方表单添加，或使用「定时」页创建独立提醒。</p>
        </div>
      ) : (
        <div className="data-table-wrap">
          <table className="data-table todo-table">
            <thead>
              <tr>
                <th className="todo-table__check" aria-label="完成" />
                <th>标题</th>
                <th>备注</th>
                <th>截止</th>
                <th aria-label="定时" />
                <th aria-label="操作" />
              </tr>
            </thead>
            <tbody>
              {items.map((row) => (
                <tr
                  key={row.id}
                  className={isOverdue(row) ? "todo-row--overdue" : undefined}
                >
                  <td>
                    <input
                      type="checkbox"
                      className="todo-check"
                      checked={row.completed}
                      onChange={() => void toggleDone(row)}
                      aria-label={row.completed ? "标记未完成" : "标记完成"}
                    />
                  </td>
                  <td>
                    <span className={row.completed ? "cell-muted" : undefined}>{row.title}</span>
                  </td>
                  <td className="cell-muted cell-clip">{row.notes || "—"}</td>
                  <td className="cell-muted">{formatDue(row.dueAt)}</td>
                  <td>
                    <Link
                      className="todo-schedule-link"
                      to={`/todo/schedules?todoId=${encodeURIComponent(row.id)}`}
                      title="为此条目添加定时提醒"
                    >
                      <Bell size={16} aria-hidden />
                      <span>定时</span>
                    </Link>
                  </td>
                  <td>
                    <div className="todo-row__actions">
                      <button type="button" className="btn-secondary btn-small" onClick={() => startEdit(row)}>
                        编辑
                      </button>
                      <button type="button" className="btn-secondary btn-small" onClick={() => void remove(row.id)}>
                        删除
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  );
}
