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
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
    },
};
use tauri::AppHandle;

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
    pub downloads: DownloadRegistry,
}

impl AppState {
    pub fn new(data_dir: PathBuf, app_handle: AppHandle) -> Result<Self, String> {
        let settings_store = SettingsStore::new(&data_dir);
        let settings = settings_store
            .load_or_default()
            .map_err(|error| error.to_string())?;
        let models = ModelCatalog::new(&data_dir);
        let transcription = WhisperTranscriptionProvider::new(models.clone());
        let hotkeys = HotkeyService::new(app_handle);
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
            downloads: DownloadRegistry::default(),
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

#[derive(Default)]
pub struct DownloadRegistry {
    active: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl DownloadRegistry {
    pub fn start(&self, model_id: &str) -> Result<Arc<AtomicBool>, String> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| "download registry lock was poisoned".to_string())?;

        if active.contains_key(model_id) {
            return Err(format!("model download already active: {model_id}"));
        }

        let canceled = Arc::new(AtomicBool::new(false));
        active.insert(model_id.to_string(), Arc::clone(&canceled));
        Ok(canceled)
    }

    pub fn cancel(&self, model_id: &str) -> Result<bool, String> {
        let active = self
            .active
            .lock()
            .map_err(|_| "download registry lock was poisoned".to_string())?;

        if let Some(canceled) = active.get(model_id) {
            canceled.store(true, Ordering::Relaxed);
            return Ok(true);
        }

        Ok(false)
    }

    pub fn finish(&self, model_id: &str) {
        if let Ok(mut active) = self.active.lock() {
            active.remove(model_id);
        }
    }
}
