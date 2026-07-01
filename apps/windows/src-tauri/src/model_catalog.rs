use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelDescriptor {
    pub id: String,
    pub name: String,
    pub file_name: String,
    pub download_url: String,
    pub size_mb: u64,
    pub recommended_min_ram_gb: u64,
    pub languages: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelCacheStatus {
    pub model: ModelDescriptor,
    pub is_downloaded: bool,
    pub path: PathBuf,
    pub bytes_on_disk: u64,
}

#[derive(Debug, Error)]
pub enum ModelCatalogError {
    #[error("unknown model id: {0}")]
    UnknownModel(String),
    #[error("failed to inspect model cache: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct ModelCatalog {
    model_dir: PathBuf,
}

impl ModelCatalog {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            model_dir: data_dir.into().join("models"),
        }
    }

    pub fn model_dir(&self) -> &Path {
        &self.model_dir
    }

    pub fn list(&self) -> Result<Vec<ModelCacheStatus>, ModelCatalogError> {
        catalog()
            .into_iter()
            .map(|model| self.status_for_descriptor(model))
            .collect()
    }

    pub fn status(&self, id: &str) -> Result<ModelCacheStatus, ModelCatalogError> {
        let model = catalog()
            .into_iter()
            .find(|model| model.id == id)
            .ok_or_else(|| ModelCatalogError::UnknownModel(id.to_string()))?;
        self.status_for_descriptor(model)
    }

    pub fn model_path(&self, id: &str) -> Result<PathBuf, ModelCatalogError> {
        Ok(self.status(id)?.path)
    }

    pub fn clear_cache(&self, id: Option<String>) -> Result<(), ModelCatalogError> {
        if let Some(id) = id {
            let path = self.model_path(&id)?;
            if path.exists() {
                fs::remove_file(path)?;
            }
            return Ok(());
        }

        if self.model_dir.exists() {
            fs::remove_dir_all(&self.model_dir)?;
        }
        Ok(())
    }

    fn status_for_descriptor(
        &self,
        model: ModelDescriptor,
    ) -> Result<ModelCacheStatus, ModelCatalogError> {
        let path = self.model_dir.join(&model.file_name);
        let bytes_on_disk = fs::metadata(&path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);

        Ok(ModelCacheStatus {
            model,
            is_downloaded: bytes_on_disk > 0,
            path,
            bytes_on_disk,
        })
    }
}

pub fn catalog() -> Vec<ModelDescriptor> {
    vec![
        ModelDescriptor {
            id: "whisper-base.en".to_string(),
            name: "Whisper Base English".to_string(),
            file_name: "ggml-base.en.bin".to_string(),
            download_url:
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin"
                    .to_string(),
            size_mb: 142,
            recommended_min_ram_gb: 4,
            languages: "English".to_string(),
        },
        ModelDescriptor {
            id: "whisper-small.en".to_string(),
            name: "Whisper Small English".to_string(),
            file_name: "ggml-small.en.bin".to_string(),
            download_url:
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin"
                    .to_string(),
            size_mb: 466,
            recommended_min_ram_gb: 6,
            languages: "English".to_string(),
        },
        ModelDescriptor {
            id: "whisper-base".to_string(),
            name: "Whisper Base Multilingual".to_string(),
            file_name: "ggml-base.bin".to_string(),
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin"
                .to_string(),
            size_mb: 142,
            recommended_min_ram_gb: 4,
            languages: "Multilingual".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_default_model() {
        let model = catalog()
            .into_iter()
            .find(|model| model.id == "whisper-base.en")
            .unwrap();

        assert_eq!(model.file_name, "ggml-base.en.bin");
    }

    #[test]
    fn status_reports_missing_file() {
        let temp_dir =
            std::env::temp_dir().join(format!("fluidvoice-models-test-{}", uuid::Uuid::new_v4()));
        let catalog = ModelCatalog::new(&temp_dir);

        let status = catalog.status("whisper-base.en").unwrap();

        assert!(!status.is_downloaded);
        assert_eq!(status.bytes_on_disk, 0);
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
