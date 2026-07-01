use crate::{
    audio::{AudioInputDevice, RecordingSnapshot},
    enhancement::{provider_key, EnhancementResult},
    history::HistoryEntry,
    model_catalog::ModelCacheStatus,
    secrets::SecretStatus,
    settings::AppSettings,
    state::AppState,
    text_insertion::InsertionResult,
    transcription::{TranscriptResult, TranscriptionProvider},
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub settings: AppSettings,
    pub devices: Vec<AudioInputDevice>,
    pub devices_error: Option<String>,
    pub models: Vec<ModelCacheStatus>,
    pub recording: Option<RecordingSnapshot>,
    pub openai_secret: SecretStatus,
    pub groq_secret: SecretStatus,
    pub custom_secret: SecretStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationResult {
    pub transcript: TranscriptResult,
    pub final_text: String,
    pub enhancement: Option<EnhancementResult>,
    pub enhancement_error: Option<String>,
    pub insertion: Option<InsertionResult>,
    pub history_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgress {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[tauri::command]
pub fn get_app_status(state: State<'_, AppState>) -> Result<AppStatus, String> {
    let settings = state.settings()?;
    let (devices, devices_error) = match state.audio.list_input_devices() {
        Ok(devices) => (devices, None),
        Err(error) => (Vec::new(), Some(error.to_string())),
    };

    Ok(AppStatus {
        settings,
        devices,
        devices_error,
        models: state.models.list().map_err(|error| error.to_string())?,
        recording: state.audio.snapshot().map_err(|error| error.to_string())?,
        openai_secret: state
            .secrets
            .exists("openai")
            .map_err(|error| error.to_string())?,
        groq_secret: state
            .secrets
            .exists("groq")
            .map_err(|error| error.to_string())?,
        custom_secret: state
            .secrets
            .exists("custom-openai-compatible")
            .map_err(|error| error.to_string())?,
    })
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    state.settings()
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    state.replace_settings(settings)
}

#[tauri::command]
pub fn list_input_devices(state: State<'_, AppState>) -> Result<Vec<AudioInputDevice>, String> {
    state
        .audio
        .list_input_devices()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn start_dictation(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<RecordingSnapshot, String> {
    let settings = state.settings()?;
    let snapshot = state
        .audio
        .start(settings.input_device_id.as_deref())
        .map_err(|error| error.to_string())?;

    set_overlay_visible(&app, true);
    Ok(snapshot)
}

#[tauri::command]
pub fn get_recording_snapshot(
    state: State<'_, AppState>,
) -> Result<Option<RecordingSnapshot>, String> {
    state.audio.snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn cancel_dictation(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    state.audio.cancel().map_err(|error| error.to_string())?;
    set_overlay_visible(&app, false);
    Ok(())
}

#[tauri::command]
pub async fn finish_dictation(
    app: AppHandle,
    state: State<'_, AppState>,
    insert: bool,
) -> Result<DictationResult, String> {
    let settings = state.settings()?;
    let audio = state.audio.stop().map_err(|error| error.to_string())?;
    set_overlay_visible(&app, false);

    let transcript = state
        .transcription
        .transcribe(&audio, &settings)
        .map_err(|error| error.to_string())?;

    let mut final_text = transcript.raw_text.clone();
    let mut enhancement = None;
    let mut enhancement_error = None;

    if settings.enhancement.enabled {
        let key = provider_key(&settings.enhancement.provider);
        let secret = state.secrets.get(key).map_err(|error| error.to_string())?;
        match state
            .enhancement
            .enhance(&transcript.raw_text, &settings, secret)
            .await
        {
            Ok(result) => {
                final_text = result.text.clone();
                enhancement = Some(result);
            }
            Err(error) => {
                enhancement_error = Some(error.to_string());
            }
        }
    }

    let insertion = if insert {
        Some(
            state
                .insertion
                .insert(&final_text, &settings.insertion)
                .map_err(|error| error.to_string())?,
        )
    } else {
        None
    };

    let mut history_id = None;
    if settings.history.enabled {
        let entry = HistoryEntry::new(
            transcript.raw_text.clone(),
            final_text.clone(),
            transcript.model_id.clone(),
            enhancement.as_ref().map(|result| result.provider.clone()),
            insertion
                .as_ref()
                .map(|result| result.success)
                .unwrap_or(false),
        );
        history_id = Some(entry.id);
        state
            .history_store
            .append(entry, settings.history.max_items)
            .map_err(|error| error.to_string())?;
    }

    Ok(DictationResult {
        transcript,
        final_text,
        enhancement,
        enhancement_error,
        insertion,
        history_id,
    })
}

#[tauri::command]
pub fn list_models(state: State<'_, AppState>) -> Result<Vec<ModelCacheStatus>, String> {
    state.models.list().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn validate_model_cache(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<ModelCacheStatus, String> {
    state
        .models
        .status(&model_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<ModelCacheStatus, String> {
    let status = state
        .models
        .status(&model_id)
        .map_err(|error| error.to_string())?;
    if status.is_downloaded {
        return Ok(status);
    }

    tokio::fs::create_dir_all(state.models.model_dir())
        .await
        .map_err(|error| error.to_string())?;

    let target_path = status.path.clone();
    let part_path = target_path.with_extension("part");
    let response = reqwest::Client::new()
        .get(&status.model.download_url)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let response = response
        .error_for_status()
        .map_err(|error| error.to_string())?;
    let total_bytes = response.content_length();
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(&part_path)
        .await
        .map_err(|error| error.to_string())?;
    let mut downloaded_bytes = 0_u64;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| error.to_string())?;
        downloaded_bytes += chunk.len() as u64;
        file.write_all(&chunk)
            .await
            .map_err(|error| error.to_string())?;
        let _ = app.emit(
            "model-download-progress",
            ModelDownloadProgress {
                model_id: model_id.clone(),
                downloaded_bytes,
                total_bytes,
            },
        );
    }

    file.flush().await.map_err(|error| error.to_string())?;
    tokio::fs::rename(&part_path, &target_path)
        .await
        .map_err(|error| error.to_string())?;

    state
        .models
        .status(&model_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_model_cache(
    state: State<'_, AppState>,
    model_id: Option<String>,
) -> Result<Vec<ModelCacheStatus>, String> {
    state
        .models
        .clear_cache(model_id)
        .map_err(|error| error.to_string())?;
    state.models.list().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn insert_text(state: State<'_, AppState>, text: String) -> Result<InsertionResult, String> {
    let settings = state.settings()?;
    state
        .insertion
        .insert(&text, &settings.insertion)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_history(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<HistoryEntry>, String> {
    let settings = state.settings()?;
    state
        .history_store
        .list(limit.unwrap_or(settings.history.max_items))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    state
        .history_store
        .clear()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_api_key(
    state: State<'_, AppState>,
    key: String,
    secret: String,
) -> Result<SecretStatus, String> {
    state
        .secrets
        .set(&key, &secret)
        .map_err(|error| error.to_string())?;
    state
        .secrets
        .exists(&key)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn has_api_key(state: State<'_, AppState>, key: String) -> Result<SecretStatus, String> {
    state
        .secrets
        .exists(&key)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_api_key(state: State<'_, AppState>, key: String) -> Result<SecretStatus, String> {
    state
        .secrets
        .delete(&key)
        .map_err(|error| error.to_string())?;
    state
        .secrets
        .exists(&key)
        .map_err(|error| error.to_string())
}

fn set_overlay_visible(app: &AppHandle, visible: bool) {
    if let Some(window) = app.get_webview_window("overlay") {
        if visible {
            let _ = window.show();
            let _ = window.set_focus();
        } else {
            let _ = window.hide();
        }
    }
}
