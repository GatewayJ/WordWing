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
          默认 <strong>Ctrl+Shift+1</strong>。保存后会立即重注册。
          在 <strong>Linux Wayland</strong> 下，若已安装桌面门户，系统可能弹出对话框要求确认「全局快捷键」绑定。
        </p>
        <p className="page-lead" style={{ marginBottom: 12 }}>
          <strong>不依赖门户时：</strong>使用 <strong>X11</strong> 会话登录桌面（全局快捷键由系统常规通道注册），或始终在{" "}
          <Link to="/english/vocabulary">生词</Link> 页点击 <strong>「打开翻译浮层（划词）」</strong>，效果与快捷键相同。
        </p>
        <p className="page-lead" style={{ marginBottom: 12 }}>
          <strong>中英翻译：</strong>固定 <strong>Ctrl+Shift+2</strong>（与数字行 <strong>2 / @</strong> 同键）。与主翻译相同：先
          <strong>划词</strong>再按（Linux PRIMARY），否则用剪贴板；自动判断译成英文或中文，浮层内可
          <strong>复制译文</strong>。与上方预设无关。
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
          <strong>划词 / 剪贴板</strong>由应用内建剪贴板接口读取（Linux：PRIMARY
          优先，再标准剪贴板；X11 与 Wayland 均支持）。若仍取不到内容，可用浮层「用剪贴板再试」。
        </p>
        <p>
          返回 <Link to="/english/vocabulary">生词</Link> 或{" "}
          <Link to="/todo/items">Todo 条目</Link>。
        </p>
      </div>
    </>
  );
}
