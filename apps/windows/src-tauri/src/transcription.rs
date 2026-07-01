use crate::{
    audio::AudioBuffer,
    model_catalog::{ModelCatalog, ModelCatalogError},
    settings::AppSettings,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptResult {
    pub raw_text: String,
    pub model_id: String,
    pub language: String,
    pub audio_duration_ms: u64,
}

pub trait TranscriptionProvider {
    fn provider_id(&self) -> &'static str;
    fn prepare(&self, settings: &AppSettings) -> Result<(), TranscriptionError>;
    fn transcribe(
        &self,
        audio: &AudioBuffer,
        settings: &AppSettings,
    ) -> Result<TranscriptResult, TranscriptionError>;
}

#[derive(Debug, Error)]
pub enum TranscriptionError {
    #[error("recording is empty")]
    EmptyAudio,
    #[error("model is not downloaded: {0}")]
    ModelNotDownloaded(String),
    #[error("model catalog error: {0}")]
    ModelCatalog(#[from] ModelCatalogError),
    #[error("local Whisper runtime is not enabled in this build")]
    WhisperRuntimeUnavailable,
    #[cfg(feature = "whisper")]
    #[error("Whisper transcription failed: {0}")]
    Whisper(String),
}

#[derive(Debug, Clone)]
pub struct WhisperTranscriptionProvider {
    catalog: ModelCatalog,
}

impl WhisperTranscriptionProvider {
    pub fn new(catalog: ModelCatalog) -> Self {
        Self { catalog }
    }
}

impl TranscriptionProvider for WhisperTranscriptionProvider {
    fn provider_id(&self) -> &'static str {
        "whisper-local"
    }

    fn prepare(&self, settings: &AppSettings) -> Result<(), TranscriptionError> {
        let status = self.catalog.status(&settings.transcription.model_id)?;
        if !status.is_downloaded {
            return Err(TranscriptionError::ModelNotDownloaded(
                settings.transcription.model_id.clone(),
            ));
        }
        Ok(())
    }

    fn transcribe(
        &self,
        audio: &AudioBuffer,
        settings: &AppSettings,
    ) -> Result<TranscriptResult, TranscriptionError> {
        if audio.samples.is_empty() {
            return Err(TranscriptionError::EmptyAudio);
        }

        self.prepare(settings)?;
        transcribe_with_whisper(audio, settings, &self.catalog)
    }
}

#[cfg(not(feature = "whisper"))]
fn transcribe_with_whisper(
    _audio: &AudioBuffer,
    _settings: &AppSettings,
    _catalog: &ModelCatalog,
) -> Result<TranscriptResult, TranscriptionError> {
    Err(TranscriptionError::WhisperRuntimeUnavailable)
}

#[cfg(feature = "whisper")]
fn transcribe_with_whisper(
    audio: &AudioBuffer,
    settings: &AppSettings,
    catalog: &ModelCatalog,
) -> Result<TranscriptResult, TranscriptionError> {
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    let model_path = catalog.model_path(&settings.transcription.model_id)?;
    let model_path = model_path
        .to_str()
        .ok_or_else(|| TranscriptionError::Whisper("model path is not valid UTF-8".to_string()))?;
    let samples = audio.to_mono_f32_16khz();
    if samples.is_empty() {
        return Err(TranscriptionError::EmptyAudio);
    }

    let context = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .map_err(|error| TranscriptionError::Whisper(error.to_string()))?;
    let mut state = context
        .create_state()
        .map_err(|error| TranscriptionError::Whisper(error.to_string()))?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    if settings.language != "auto" {
        params.set_language(Some(&settings.language));
    }
    params.set_translate(false);
    params.set_print_progress(false);
    params.set_print_special(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    state
        .full(params, &samples)
        .map_err(|error| TranscriptionError::Whisper(error.to_string()))?;

    let segment_count = state
        .full_n_segments()
        .map_err(|error| TranscriptionError::Whisper(error.to_string()))?;
    let mut raw_text = String::new();
    for segment in 0..segment_count {
        let text = state
            .full_get_segment_text(segment)
            .map_err(|error| TranscriptionError::Whisper(error.to_string()))?;
        raw_text.push_str(text.trim());
        raw_text.push(' ');
    }

    Ok(TranscriptResult {
        raw_text: raw_text.trim().to_string(),
        model_id: settings.transcription.model_id.clone(),
        language: settings.language.clone(),
        audio_duration_ms: audio.duration_ms(),
    })
}
