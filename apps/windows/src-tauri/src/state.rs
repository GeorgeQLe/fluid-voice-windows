use crate::{
    audio::AudioCaptureService,
    enhancement::EnhancementService,
    history::HistoryStore,
    hotkey::HotkeyService,
    model_catalog::ModelCatalog,
    secrets::SecretStore,
    settings::{AppSettings, SettingsStore},
    text_insertion::TextInsertionService,
    transcription::WhisperTranscriptionProvider,
};
use std::{
    path::PathBuf,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub struct AppState {
    settings: RwLock<AppSettings>,
    pub settings_store: SettingsStore,
    pub history_store: HistoryStore,
    pub models: ModelCatalog,
    pub audio: AudioCaptureService,
    pub transcription: WhisperTranscriptionProvider,
    pub enhancement: EnhancementService,
    pub hotkeys: HotkeyService,
    pub insertion: TextInsertionService,
    pub secrets: SecretStore,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Result<Self, String> {
        let settings_store = SettingsStore::new(&data_dir);
        let settings = settings_store
            .load_or_default()
            .map_err(|error| error.to_string())?;
        let models = ModelCatalog::new(&data_dir);
        let transcription = WhisperTranscriptionProvider::new(models.clone());
        let hotkeys = HotkeyService::default();
        hotkeys
            .apply(&settings.hotkey)
            .map_err(|error| error.to_string())?;

        Ok(Self {
            settings: RwLock::new(settings),
            settings_store,
            history_store: HistoryStore::new(&data_dir),
            models,
            audio: AudioCaptureService::default(),
            transcription,
            enhancement: EnhancementService::new(),
            hotkeys,
            insertion: TextInsertionService::default(),
            secrets: SecretStore::default(),
        })
    }

    pub fn settings(&self) -> Result<AppSettings, String> {
        Ok(self.read_settings()?.clone())
    }

    pub fn replace_settings(&self, settings: AppSettings) -> Result<AppSettings, String> {
        self.hotkeys
            .apply(&settings.hotkey)
            .map_err(|error| error.to_string())?;
        self.settings_store
            .save(&settings)
            .map_err(|error| error.to_string())?;

        let mut guard = self.write_settings()?;
        *guard = settings.clone();
        Ok(settings)
    }

    fn read_settings(&self) -> Result<RwLockReadGuard<'_, AppSettings>, String> {
        self.settings
            .read()
            .map_err(|_| "settings lock was poisoned".to_string())
    }

    fn write_settings(&self) -> Result<RwLockWriteGuard<'_, AppSettings>, String> {
        self.settings
            .write()
            .map_err(|_| "settings lock was poisoned".to_string())
    }
}
