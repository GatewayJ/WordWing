import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";

type HotkeyChoice = { id: string; label: string };

export function SettingsPage() {
  const [choices, setChoices] = useState<HotkeyChoice[]>([]);
  const [preset, setPreset] = useState("");
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const [c, p] = await Promise.all([
        invoke<HotkeyChoice[]>("list_translate_hotkey_choices"),
        invoke<string>("get_translate_hotkey_preset"),
      ]);
      setChoices(c);
      setPreset(p);
    } catch {
      setChoices([]);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const saveHotkey = async () => {
    setSaving(true);
    setMsg(null);
    try {
      await invoke("set_translate_hotkey_preset", { preset });
      setMsg("已保存并立即生效。");
    } catch (e) {
      setMsg(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      <h2 className="page-title">设置</h2>
      <p className="page-lead">
        翻译使用阿里云 DashScope。密钥<strong>不要</strong>写进前端，请在启动应用的终端里{" "}
        <code style={{ fontSize: 13 }}>export DASHSCOPE_API_KEY=你的密钥</code>。
      </p>

      <div className="card" style={{ maxWidth: "32rem" }}>
        <h3
          className="page-title"
          style={{ fontSize: "1.25rem", marginBottom: 12 }}
        >
          翻译快捷键
        </h3>
        <p className="page-lead" style={{ marginBottom: 12 }}>
          <strong>Ctrl+Shift+D</strong> 常与浏览器、IDE
          冲突。默认已改为 <strong>Super+Shift+T</strong>（Windows 键 + Shift +
          T）；你可从下列预设中任选其一，保存后立即重注册全局热键。
        </p>
        <label className="settings-label" htmlFor="hotkey-preset">
          预设
        </label>
        <select
          id="hotkey-preset"
          className="settings-select"
          value={preset}
          onChange={(e) => setPreset(e.target.value)}
        >
          {choices.map((c) => (
            <option key={c.id} value={c.id}>
              {c.label}
            </option>
          ))}
        </select>
        <div style={{ marginTop: 14 }}>
          <button
            type="button"
            className="settings-save-btn"
            disabled={saving || !preset}
            onClick={() => void saveHotkey()}
          >
            {saving ? "保存中…" : "保存快捷键"}
          </button>
        </div>
        {msg && (
          <p className="page-lead" style={{ marginTop: 12, marginBottom: 0 }}>
            {msg}
          </p>
        )}
      </div>

      <div className="card" style={{ marginTop: 16 }}>
        <p>
          <strong>划词</strong>依赖 X11 与 <code>xclip</code>（PRIMARY /
          剪贴板）。Wayland 下若不可用，请用浮层「用剪贴板再试」。
        </p>
        <p>
          返回 <Link to="/english/vocabulary">生词</Link> 或{" "}
          <Link to="/todo/items">Todo 条目</Link>。
        </p>
      </div>
    </>
  );
}
