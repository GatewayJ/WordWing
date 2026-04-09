use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

/// 默认：Ctrl+Shift+1（与物理键 1 / Shift+! 同键）。
pub const DEFAULT_TRANSLATE_HOTKEY_PRESET: &str = "ctrl_shift_1";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SettingsFile {
    #[serde(default = "default_preset")]
    translate_hotkey_preset: String,
}

fn default_preset() -> String {
    DEFAULT_TRANSLATE_HOTKEY_PRESET.to_string()
}

pub struct AppSettings {
    tree: sled::Tree,
    inner: Mutex<String>,
}

impl AppSettings {
    pub fn open(db: &sled::Db) -> Result<Self, String> {
        let tree = db.open_tree("settings").map_err(|e| e.to_string())?;
        let preset = match tree.get(b"app").map_err(|e| e.to_string())? {
            None => DEFAULT_TRANSLATE_HOTKEY_PRESET.to_string(),
            Some(raw) => {
                let s: SettingsFile = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
                validate_preset_id(&s.translate_hotkey_preset)?;
                s.translate_hotkey_preset
            }
        };
        Ok(Self {
            tree,
            inner: Mutex::new(preset),
        })
    }

    pub fn preset(&self) -> String {
        self.inner
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|_| DEFAULT_TRANSLATE_HOTKEY_PRESET.to_string())
    }

    pub fn set_preset(&self, preset: &str) -> Result<(), String> {
        validate_preset_id(preset)?;
        let mut g = self.inner.lock().map_err(|e| e.to_string())?;
        *g = preset.to_string();
        let file = SettingsFile {
            translate_hotkey_preset: preset.to_string(),
        };
        let raw = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
        self.tree
            .insert(b"app", raw.as_bytes())
            .map_err(|e| e.to_string())?;
        self.tree.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub fn validate_preset_id(preset: &str) -> Result<(), String> {
    preset_to_shortcut(preset).map(|_| ())
}

/// GTK / XDG GlobalShortcuts 门户使用的 accelerator 字符串（Wayland）。
/// 参见 GTK `gtk_accelerator_parse` 风格：`<Control>`、`<Shift>`、`<Alt>`、`<Super>`。
pub fn preset_to_portal_preferred_trigger(preset: &str) -> Option<&'static str> {
    match preset {
        "ctrl_shift_1" => Some("<Control><Shift>1"),
        "meta_shift_t" => Some("<Super><Shift>t"),
        "ctrl_shift_d" => Some("<Control><Shift>d"),
        "ctrl_alt_shift_t" => Some("<Control><Alt><Shift>t"),
        "alt_shift_t" => Some("<Alt><Shift>t"),
        "ctrl_shift_y" => Some("<Control><Shift>y"),
        _ => None,
    }
}

pub fn preset_to_shortcut(preset: &str) -> Result<Shortcut, String> {
    match preset {
        "meta_shift_t" => Ok(Shortcut::new(
            Some(Modifiers::SUPER | Modifiers::SHIFT),
            Code::KeyT,
        )),
        "ctrl_shift_d" => Ok(Shortcut::new(
            Some(Modifiers::CONTROL | Modifiers::SHIFT),
            Code::KeyD,
        )),
        "ctrl_alt_shift_t" => Ok(Shortcut::new(
            Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT),
            Code::KeyT,
        )),
        "alt_shift_t" => Ok(Shortcut::new(
            Some(Modifiers::ALT | Modifiers::SHIFT),
            Code::KeyT,
        )),
        "ctrl_shift_y" => Ok(Shortcut::new(
            Some(Modifiers::CONTROL | Modifiers::SHIFT),
            Code::KeyY,
        )),
        "ctrl_shift_1" => Ok(Shortcut::new(
            Some(Modifiers::CONTROL | Modifiers::SHIFT),
            Code::Digit1,
        )),
        _ => Err(format!("未知的热键预设: {}", preset)),
    }
}

pub fn preset_display_label(preset: &str) -> &'static str {
    match preset {
        "meta_shift_t" => "Super + Shift + T",
        "ctrl_shift_d" => "Ctrl + Shift + D",
        "ctrl_alt_shift_t" => "Ctrl + Alt + Shift + T",
        "alt_shift_t" => "Alt + Shift + T",
        "ctrl_shift_y" => "Ctrl + Shift + Y",
        "ctrl_shift_1" => "Ctrl + Shift + 1（与 1 / ! 同键，推荐）",
        _ => "自定义",
    }
}

#[derive(Clone, Serialize)]
pub struct HotkeyChoice {
    pub id: String,
    pub label: String,
}

pub fn all_hotkey_choices() -> Vec<HotkeyChoice> {
    [
        "ctrl_shift_1",
        "meta_shift_t",
        "ctrl_shift_d",
        "ctrl_alt_shift_t",
        "alt_shift_t",
        "ctrl_shift_y",
    ]
    .iter()
    .map(|id| HotkeyChoice {
        id: (*id).to_string(),
        label: preset_display_label(id).to_string(),
    })
    .collect()
}
