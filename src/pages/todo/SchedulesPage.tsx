import { useCallback, useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { PlainDateTimeFields } from "../../components/PlainDateTimeFields";
import { dateTimePartsToIso, validateDatePart, validateTimePart } from "../../lib/datetimeInput";

type TodoItem = {
  id: string;
  title: string;
  notes: string;
  dueAt: string | null;
  completed: boolean;
  createdAt: string;
};

type TodoSchedule = {
  id: string;
  todoId: string | null;
  title: string;
  fireAt: string;
  createdAt: string;
  /** 是否已推送系统通知 */
  notificationSent?: boolean;
};

function formatFire(iso: string) {
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

export function SchedulesPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const todoIdFromUrl = searchParams.get("todoId");

  const [items, setItems] = useState<TodoItem[]>([]);
  const [schedules, setSchedules] = useState<TodoSchedule[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [fireDateStr, setFireDateStr] = useState("");
  const [fireTimeStr, setFireTimeStr] = useState("");
  const [linkTodoId, setLinkTodoId] = useState<string>("");
  const [saving, setSaving] = useState(false);

  const loadAll = useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      const [todos, sch] = await Promise.all([
        invoke<TodoItem[]>("list_todo_items"),
        invoke<TodoSchedule[]>("list_todo_schedules"),
      ]);
      setItems(todos);
      setSchedules(sch);
    } catch (e) {
      setErr(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadAll();
  }, [loadAll]);

  useEffect(() => {
    let u1: UnlistenFn | undefined;
    let u2: UnlistenFn | undefined;
    void listen("todo-schedules-changed", () => {
      void loadAll();
    }).then((fn) => {
      u1 = fn;
    });
    void listen("todo-changed", () => {
      void loadAll();
    }).then((fn) => {
      u2 = fn;
    });
    return () => {
      void u1?.();
      void u2?.();
    };
  }, [loadAll]);

  // 从「条目」带 ?todoId= 跳转时：预选关联并建议标题
  useEffect(() => {
    if (!todoIdFromUrl || items.length === 0) return;
    const found = items.find((x) => x.id === todoIdFromUrl);
    if (found) {
      setLinkTodoId(found.id);
      setTitle((t) => (t.trim() ? t : found.title));
    }
  }, [todoIdFromUrl, items]);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    const t = title.trim();
    if (!t) {
      setErr("请填写提醒标题");
      return;
    }
    if (!validateDatePart(fireDateStr) || !validateTimePart(fireTimeStr)) {
      setErr("触发时间格式有误：日期为 YYYY-MM-DD，时间为 HH:MM（24 小时）。");
      return;
    }
    const fireAt = dateTimePartsToIso(fireDateStr, fireTimeStr);
    if (!fireAt) {
      setErr("触发时间无效，请检查日期是否存在。");
      return;
    }
    setSaving(true);
    setErr(null);
    try {
      await invoke("add_todo_schedule", {
        title: t,
        fireAt,
        todoId: linkTodoId || null,
      });
      setTitle("");
      setFireDateStr("");
      setFireTimeStr("");
      if (!todoIdFromUrl) {
        setLinkTodoId("");
      }
      setSearchParams((prev) => {
        const n = new URLSearchParams(prev);
        n.delete("todoId");
        return n;
      });
      await loadAll();
    } catch (e) {
      setErr(String(e));
    } finally {
      setSaving(false);
    }
  };

  const remove = async (id: string) => {
    if (!window.confirm("确定删除该定时？")) return;
    try {
      await invoke("delete_todo_schedule", { id });
      await loadAll();
    } catch (e) {
      setErr(String(e));
    }
  };

  const linkLabel = (sch: TodoSchedule) => {
    if (!sch.todoId) return "独立提醒";
    const it = items.find((x) => x.id === sch.todoId);
    return it ? `条目：${it.title}` : `条目（${sch.todoId.slice(0, 8)}…）`;
  };

  return (
    <>
      <h2 className="page-title">定时</h2>
      <p className="page-lead">
        在此添加<strong>触发时间</strong>与标题；可选关联{" "}
        <Link to="/todo/items">条目</Link>，或作为<strong>独立提醒</strong>。列表按触发时间升序排列。
      </p>
      <p className="page-lead cell-muted" style={{ fontSize: 14 }}>
        应用运行期间约每 30 秒检查一次到期定时，通过<strong>系统通知</strong>提醒（每条仅推送一次）。请允许通知权限；Linux 需已安装{" "}
        <code style={{ fontSize: 13 }}>libnotify</code> 等（一般桌面已带）。
      </p>

      {err && (
        <p className="page-lead" style={{ color: "var(--error)" }}>
          {err}
        </p>
      )}

      <div className="card todo-form-card">
        <h3 className="todo-form-card__title">新建定时</h3>
        <form className="todo-form" onSubmit={(e) => void submit(e)}>
          <label className="settings-label" htmlFor="sch-title">
            提醒标题
          </label>
          <input
            id="sch-title"
            className="todo-input"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="必填"
            autoComplete="off"
          />
          <p className="settings-label" style={{ marginBottom: 4 }}>
            触发时间
          </p>
          <PlainDateTimeFields
            idPrefix="sch-fire"
            dateStr={fireDateStr}
            timeStr={fireTimeStr}
            onDateStrChange={setFireDateStr}
            onTimeStrChange={setFireTimeStr}
            dateLabel="日期"
            timeLabel="时间"
            optionalHint="必填。文本输入，避免系统日历弹层阻塞界面。"
          />
          <label className="settings-label" htmlFor="sch-link">
            关联条目（可选）
          </label>
          <select
            id="sch-link"
            className="settings-select"
            value={linkTodoId}
            onChange={(e) => setLinkTodoId(e.target.value)}
          >
            <option value="">独立提醒（不关联条目）</option>
            {items
              .filter((x) => !x.completed)
              .map((x) => (
                <option key={x.id} value={x.id}>
                  {x.title}
                </option>
              ))}
          </select>
          <div className="todo-form__actions">
            <button type="submit" className="settings-save-btn" disabled={saving}>
              {saving ? "保存中…" : "添加定时"}
            </button>
          </div>
        </form>
      </div>

      {loading ? (
        <p className="page-lead">加载中…</p>
      ) : schedules.length === 0 ? (
        <div className="card">
          <p>暂无定时。可在上方添加，或从「条目」行内点「定时」预填关联。</p>
        </div>
      ) : (
        <div className="data-table-wrap">
          <table className="data-table todo-table">
            <thead>
              <tr>
                <th>触发时间</th>
                <th>标题</th>
                <th>关联</th>
                <th>通知</th>
                <th aria-label="操作" />
              </tr>
            </thead>
            <tbody>
              {schedules.map((row) => (
                <tr key={row.id}>
                  <td>{formatFire(row.fireAt)}</td>
                  <td>{row.title}</td>
                  <td className="cell-muted">
                    {row.todoId ? (
                      <Link to="/todo/items">{linkLabel(row)}</Link>
                    ) : (
                      "独立提醒"
                    )}
                  </td>
                  <td className="cell-muted">
                    {row.notificationSent ? "已推送" : new Date(row.fireAt).getTime() <= Date.now() ? "待推送" : "—"}
                  </td>
                  <td>
                    <button type="button" className="btn-secondary btn-small" onClick={() => void remove(row.id)}>
                      删除
                    </button>
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
