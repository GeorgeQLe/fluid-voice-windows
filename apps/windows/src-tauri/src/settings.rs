use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

pub const CURRENT_SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub version: u32,
    pub language: String,
    pub input_device_id: Option<String>,
    pub hotkey: HotkeySettings,
    pub transcription: TranscriptionSettings,
    pub enhancement: EnhancementSettings,
    pub insertion: TextInsertionSettings,
    pub history: HistorySettings,
    pub startup: StartupSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: CURRENT_SETTINGS_VERSION,
            language: "auto".to_string(),
            input_device_id: None,
            hotkey: HotkeySettings::default(),
            transcription: TranscriptionSettings::default(),
            enhancement: EnhancementSettings::default(),
            insertion: TextInsertionSettings::default(),
            history: HistorySettings::default(),
            startup: StartupSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct HotkeySettings {
    pub enabled: bool,
    pub mode: HotkeyMode,
    pub shortcut: String,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: HotkeyMode::Toggle,
            shortcut: "Ctrl+Alt+Space".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyMode {
    Toggle,
    Hold,
}

impl Default for HotkeyMode {
    fn default() -> Self {
        Self::Toggle
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct TranscriptionSettings {
    pub provider: TranscriptionProviderKind,
    pub model_id: String,
    pub streaming_preview: bool,
    pub max_recording_seconds: u32,
}

impl Default for TranscriptionSettings {
    fn default() -> Self {
        Self {
            provider: TranscriptionProviderKind::WhisperLocal,
            model_id: "whisper-base.en".to_string(),
            streaming_preview: true,
            max_recording_seconds: 90,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TranscriptionProviderKind {
    WhisperLocal,
}

impl Default for TranscriptionProviderKind {
    fn default() -> Self {
        Self::WhisperLocal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct EnhancementSettings {
    pub enabled: bool,
    pub provider: EnhancementProvider,
    pub base_url: String,
    pub model: String,
    pub prompt_profile: PromptProfile,
    pub timeout_seconds: u64,
}

impl Default for EnhancementSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: EnhancementProvider::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4.1-mini".to_string(),
            prompt_profile: PromptProfile::Default,
            timeout_seconds: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum EnhancementProvider {
    OpenAi,
    Groq,
    CustomOpenAiCompatible,
}

impl Default for EnhancementProvider {
    fn default() -> Self {
        Self::OpenAi
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PromptProfile {
    Default,
    CleanTranscript,
    Email,
    CodeComments,
}

impl Default for PromptProfile {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct TextInsertionSettings {
    pub mode: TextInsertionMode,
    pub typing_delay_ms: u64,
    pub restore_clipboard: bool,
}

impl Default for TextInsertionSettings {
    fn default() -> Self {
        Self {
            mode: TextInsertionMode::SendInput,
            typing_delay_ms: 0,
            restore_clipboard: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TextInsertionMode {
    SendInput,
    ClipboardFallback,
}

impl Default for TextInsertionMode {
    fn default() -> Self {
        Self::SendInput
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct HistorySettings {
    pub enabled: bool,
    pub retain_audio: bool,
    pub max_items: usize,
}

impl Default for HistorySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            retain_audio: false,
            max_items: 200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct StartupSettings {
    pub launch_at_login: bool,
}

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("failed to create settings directory: {0}")]
    CreateDir(#[source] std::io::Error),
    #[error("failed to read settings file: {0}")]
    Read(#[source] std::io::Error),
    #[error("failed to parse settings file: {0}")]
    Parse(#[source] serde_json::Error),
    #[error("failed to serialize settings file: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("failed to write settings file: {0}")]
    Write(#[source] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            path: data_dir.into().join("settings.json"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_or_default(&self) -> Result<AppSettings, SettingsError> {
        if !self.path.exists() {
            let settings = AppSettings::default();
            self.save(&settings)?;
            return Ok(settings);
        }

        let raw = fs::read_to_string(&self.path).map_err(SettingsError::Read)?;
        let value =
            serde_json::from_str::<serde_json::Value>(&raw).map_err(SettingsError::Parse)?;
        let mut settings =
            serde_json::from_value::<AppSettings>(value).map_err(SettingsError::Parse)?;
        settings = migrate(settings);
        self.save(&settings)?;
        Ok(settings)
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), SettingsError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(SettingsError::CreateDir)?;
        }

        let mut next = settings.clone();
        next.version = CURRENT_SETTINGS_VERSION;

        let raw = serde_json::to_string_pretty(&next).map_err(SettingsError::Serialize)?;
        fs::write(&self.path, raw).map_err(SettingsError::Write)
    }
}

fn migrate(mut settings: AppSettings) -> AppSettings {
    if settings.version == 0 || settings.version > CURRENT_SETTINGS_VERSION {
        settings.version = CURRENT_SETTINGS_VERSION;
    }

    if settings.transcription.max_recording_seconds == 0 {
        settings.transcription.max_recording_seconds =
            TranscriptionSettings::default().max_recording_seconds;
    }

    if settings.history.max_items == 0 {
        settings.history.max_items = HistorySettings::default().max_items;
    }

    if settings.hotkey.mode == HotkeyMode::Hold {
        settings.hotkey.mode = HotkeyMode::Toggle;
    }

    settings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_settings_fields_migrate_to_defaults() {
        let value = serde_json::json!({
            "version": 0,
            "language": "en"
        });

        let settings = migrate(serde_json::from_value(value).unwrap());

        assert_eq!(settings.version, CURRENT_SETTINGS_VERSION);
        assert_eq!(settings.language, "en");
        assert_eq!(settings.hotkey.shortcut, "Ctrl+Alt+Space");
        assert_eq!(settings.transcription.model_id, "whisper-base.en");
        assert_eq!(settings.history.max_items, 200);
    }

    #[test]
    fn save_forces_current_version() {
        let mut settings = AppSettings::default();
        settings.version = 99;

        let temp_dir =
            std::env::temp_dir().join(format!("fluidvoice-settings-test-{}", uuid::Uuid::new_v4()));
        let store = SettingsStore::new(&temp_dir);
        store.save(&settings).unwrap();
        let loaded = store.load_or_default().unwrap();

        assert_eq!(loaded.version, CURRENT_SETTINGS_VERSION);
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
