use crate::settings::{TextInsertionMode, TextInsertionSettings};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InsertionResult {
    pub attempted_mode: TextInsertionMode,
    pub success: bool,
    pub inserted_characters: usize,
    pub fallback_used: bool,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum TextInsertionError {
    #[error("text insertion is only available on Windows in this build")]
    UnsupportedPlatform,
    #[error("SendInput failed after sending {sent} of {expected} input events")]
    SendInputFailed { sent: u32, expected: u32 },
    #[error("clipboard fallback is not implemented yet")]
    ClipboardFallbackUnavailable,
}

#[derive(Debug, Clone, Default)]
pub struct TextInsertionService;

impl TextInsertionService {
    pub fn insert(
        &self,
        text: &str,
        settings: &TextInsertionSettings,
    ) -> Result<InsertionResult, TextInsertionError> {
        match settings.mode {
            TextInsertionMode::SendInput => send_input_text(text).map(|inserted| InsertionResult {
                attempted_mode: TextInsertionMode::SendInput,
                success: true,
                inserted_characters: inserted,
                fallback_used: false,
                message: "Inserted with SendInput".to_string(),
            }),
            TextInsertionMode::ClipboardFallback => {
                Err(TextInsertionError::ClipboardFallbackUnavailable)
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn send_input_text(text: &str) -> Result<usize, TextInsertionError> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        KEYEVENTF_UNICODE, VIRTUAL_KEY,
    };

    let mut inputs = Vec::with_capacity(text.encode_utf16().count() * 2);

    for unit in text.encode_utf16() {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: unit,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: unit,
                    dwFlags: KEYBD_EVENT_FLAGS(KEYEVENTF_UNICODE.0 | KEYEVENTF_KEYUP.0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }

    let expected = inputs.len() as u32;
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent != expected {
        return Err(TextInsertionError::SendInputFailed { sent, expected });
    }

    Ok(text.chars().count())
}

#[cfg(not(target_os = "windows"))]
fn send_input_text(_text: &str) -> Result<usize, TextInsertionError> {
    Err(TextInsertionError::UnsupportedPlatform)
}
