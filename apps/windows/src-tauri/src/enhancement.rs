use crate::settings::{AppSettings, EnhancementProvider, PromptProfile};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EnhancementResult {
    pub text: String,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Error)]
pub enum EnhancementError {
    #[error("enhancement is disabled")]
    Disabled,
    #[error("missing API key for {0}")]
    MissingApiKey(String),
    #[error("failed to call enhancement provider: {0}")]
    Request(#[from] reqwest::Error),
    #[error("enhancement provider returned {status}: {body}")]
    Provider { status: StatusCode, body: String },
    #[error("enhancement provider returned no choices")]
    EmptyResponse,
}

#[derive(Debug, Clone, Default)]
pub struct EnhancementService {
    client: reqwest::Client,
}

impl EnhancementService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn enhance(
        &self,
        transcript: &str,
        settings: &AppSettings,
        api_key: Option<String>,
    ) -> Result<EnhancementResult, EnhancementError> {
        if !settings.enhancement.enabled {
            return Err(EnhancementError::Disabled);
        }

        let provider = provider_key(&settings.enhancement.provider);
        let api_key =
            api_key.ok_or_else(|| EnhancementError::MissingApiKey(provider.to_string()))?;
        let base_url = settings.enhancement.base_url.trim_end_matches('/');
        let request = ChatCompletionRequest {
            model: settings.enhancement.model.clone(),
            temperature: 0.1,
            messages: render_prompt(&settings.enhancement.prompt_profile, transcript),
        };

        let response = self
            .client
            .post(format!("{base_url}/chat/completions"))
            .bearer_auth(api_key)
            .timeout(std::time::Duration::from_secs(
                settings.enhancement.timeout_seconds.max(1),
            ))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(EnhancementError::Provider { status, body });
        }

        let response = response.json::<ChatCompletionResponse>().await?;
        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or(EnhancementError::EmptyResponse)?;

        Ok(EnhancementResult {
            text: choice.message.content.trim().to_string(),
            provider: provider.to_string(),
            model: settings.enhancement.model.clone(),
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatCompletionRequest {
    model: String,
    temperature: f32,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatChoice {
    message: ChatMessage,
}

pub fn provider_key(provider: &EnhancementProvider) -> &'static str {
    match provider {
        EnhancementProvider::OpenAi => "openai",
        EnhancementProvider::Groq => "groq",
        EnhancementProvider::CustomOpenAiCompatible => "custom-openai-compatible",
    }
}

fn render_prompt(profile: &PromptProfile, transcript: &str) -> Vec<ChatMessage> {
    let system = match profile {
        PromptProfile::Default => {
            "Clean up dictated text while preserving meaning, speaker intent, and technical terms."
        }
        PromptProfile::CleanTranscript => {
            "Return a polished transcript with punctuation, capitalization, and obvious filler removed."
        }
        PromptProfile::Email => {
            "Rewrite the dictated text as a concise email draft. Preserve names, dates, and requested actions."
        }
        PromptProfile::CodeComments => {
            "Rewrite the dictated text as clear code comments or developer notes. Preserve identifiers exactly."
        }
    };

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: system.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: transcript.to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_keys_match_secret_slots() {
        assert_eq!(provider_key(&EnhancementProvider::OpenAi), "openai");
        assert_eq!(provider_key(&EnhancementProvider::Groq), "groq");
        assert_eq!(
            provider_key(&EnhancementProvider::CustomOpenAiCompatible),
            "custom-openai-compatible"
        );
    }

    #[test]
    fn prompt_keeps_transcript_as_user_message() {
        let messages = render_prompt(&PromptProfile::Email, "send this tomorrow");

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[1].content, "send this tomorrow");
    }
}
