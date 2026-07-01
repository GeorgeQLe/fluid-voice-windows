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
    #[error("clipboard fallback is only available on Windows in this build")]
    ClipboardUnsupportedPlatform,
    #[error("clipboard fallback failed while {context}: {source}")]
    Clipboard {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("SendInput failed ({primary}); clipboard fallback also failed ({fallback})")]
    FallbackAfterSendInputFailed { primary: String, fallback: String },
}

#[derive(Debug, Clone, Default)]
pub struct TextInsertionService;

impl TextInsertionService {
    pub fn insert(
        &self,
        text: &str,
        settings: &TextInsertionSettings,
    ) -> Result<InsertionResult, TextInsertionError> {
        if text.is_empty() {
            return Ok(InsertionResult {
                attempted_mode: settings.mode.clone(),
                success: true,
                inserted_characters: 0,
                fallback_used: false,
                message: "No text to insert".to_string(),
            });
        }

        match settings.mode {
            TextInsertionMode::SendInput => match send_input_text(text, settings.typing_delay_ms) {
                Ok(inserted) => Ok(InsertionResult {
                    attempted_mode: TextInsertionMode::SendInput,
                    success: true,
                    inserted_characters: inserted,
                    fallback_used: false,
                    message: "Inserted with SendInput".to_string(),
                }),
                Err(primary) => clipboard_paste_text(text, settings)
                    .map(|mut result| {
                        result.attempted_mode = TextInsertionMode::SendInput;
                        result.fallback_used = true;
                        result.message =
                            format!("SendInput failed ({primary}); {}", result.message);
                        result
                    })
                    .map_err(
                        |fallback| TextInsertionError::FallbackAfterSendInputFailed {
                            primary: primary.to_string(),
                            fallback: fallback.to_string(),
                        },
                    ),
            },
            TextInsertionMode::ClipboardFallback => clipboard_paste_text(text, settings),
        }
    }
}

#[cfg(target_os = "windows")]
fn send_input_text(text: &str, typing_delay_ms: u64) -> Result<usize, TextInsertionError> {
    use std::time::Duration;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    };

    let delay = Duration::from_millis(typing_delay_ms);

    for unit in text.encode_utf16() {
        let inputs = [unicode_input(unit, 0), unicode_input(unit, KEYEVENTF_KEYUP)];
        send_input_events(&inputs)?;

        if !delay.is_zero() {
            std::thread::sleep(delay);
        }
    }

    fn unicode_input(unit: u16, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: 0,
                    wScan: unit,
                    dwFlags: KEYEVENTF_UNICODE | flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    Ok(text.chars().count())
}

#[cfg(not(target_os = "windows"))]
fn send_input_text(_text: &str, _typing_delay_ms: u64) -> Result<usize, TextInsertionError> {
    Err(TextInsertionError::UnsupportedPlatform)
}

#[cfg(target_os = "windows")]
fn clipboard_paste_text(
    text: &str,
    settings: &TextInsertionSettings,
) -> Result<InsertionResult, TextInsertionError> {
    let previous_text = if settings.restore_clipboard {
        read_clipboard_text()?
    } else {
        None
    };

    set_clipboard_text(text)?;
    send_paste_shortcut()?;

    let mut message = "Inserted with clipboard paste".to_string();
    if settings.restore_clipboard {
        std::thread::sleep(std::time::Duration::from_millis(
            settings.typing_delay_ms.max(120),
        ));

        if let Some(previous_text) = previous_text {
            set_clipboard_text(&previous_text)?;
            message.push_str("; restored previous clipboard text");
        } else {
            message.push_str("; no prior text clipboard content was available to restore");
        }
    }

    Ok(InsertionResult {
        attempted_mode: TextInsertionMode::ClipboardFallback,
        success: true,
        inserted_characters: text.chars().count(),
        fallback_used: true,
        message,
    })
}

#[cfg(not(target_os = "windows"))]
fn clipboard_paste_text(
    _text: &str,
    _settings: &TextInsertionSettings,
) -> Result<InsertionResult, TextInsertionError> {
    Err(TextInsertionError::ClipboardUnsupportedPlatform)
}

#[cfg(target_os = "windows")]
fn send_paste_shortcut() -> Result<(), TextInsertionError> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL,
    };

    let inputs = [
        key_input(VK_CONTROL, 0),
        key_input(b'V' as u16, 0),
        key_input(b'V' as u16, KEYEVENTF_KEYUP),
        key_input(VK_CONTROL, KEYEVENTF_KEYUP),
    ];
    send_input_events(&inputs)
}

#[cfg(target_os = "windows")]
fn key_input(vk: u16, flags: u32) -> windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
    };

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(target_os = "windows")]
fn send_input_events(
    inputs: &[windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT],
) -> Result<(), TextInsertionError> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT};

    let expected = inputs.len() as u32;
    let sent = unsafe {
        SendInput(
            expected,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };

    if sent != expected {
        return Err(TextInsertionError::SendInputFailed { sent, expected });
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn read_clipboard_text() -> Result<Option<String>, TextInsertionError> {
    use windows_sys::Win32::System::{
        DataExchange::{GetClipboardData, IsClipboardFormatAvailable, CF_UNICODETEXT},
        Memory::{GlobalLock, GlobalSize, GlobalUnlock},
    };

    let _guard = ClipboardGuard::open("opening clipboard to read text")?;

    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT) == 0 {
            return Ok(None);
        }

        let handle = GetClipboardData(CF_UNICODETEXT);
        if handle == 0 as _ {
            return Err(clipboard_error("reading clipboard text handle"));
        }

        let locked = GlobalLock(handle);
        if locked.is_null() {
            return Err(clipboard_error("locking clipboard text"));
        }

        let size_bytes = GlobalSize(handle);
        let units = size_bytes / std::mem::size_of::<u16>();
        let slice = std::slice::from_raw_parts(locked as *const u16, units);
        let end = slice.iter().position(|unit| *unit == 0).unwrap_or(units);
        let text = String::from_utf16_lossy(&slice[..end]);
        let _ = GlobalUnlock(handle);

        Ok(Some(text))
    }
}

#[cfg(target_os = "windows")]
fn set_clipboard_text(text: &str) -> Result<(), TextInsertionError> {
    use windows_sys::Win32::System::{
        DataExchange::{EmptyClipboard, SetClipboardData, CF_UNICODETEXT},
        Memory::{GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
    };

    let encoded = text
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let byte_count = encoded.len() * std::mem::size_of::<u16>();

    unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, byte_count);
        if handle == 0 as _ {
            return Err(clipboard_error("allocating clipboard text"));
        }

        let locked = GlobalLock(handle);
        if locked.is_null() {
            let _ = GlobalFree(handle);
            return Err(clipboard_error("locking clipboard text for write"));
        }

        std::ptr::copy_nonoverlapping(encoded.as_ptr(), locked as *mut u16, encoded.len());
        let _ = GlobalUnlock(handle);

        let _guard = ClipboardGuard::open("opening clipboard to write text")?;
        if EmptyClipboard() == 0 {
            let _ = GlobalFree(handle);
            return Err(clipboard_error("emptying clipboard"));
        }

        if SetClipboardData(CF_UNICODETEXT, handle) == 0 as _ {
            let _ = GlobalFree(handle);
            return Err(clipboard_error("setting clipboard text"));
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
struct ClipboardGuard;

#[cfg(target_os = "windows")]
impl ClipboardGuard {
    fn open(context: &'static str) -> Result<Self, TextInsertionError> {
        use windows_sys::Win32::{Foundation::HWND, System::DataExchange::OpenClipboard};

        let opened = unsafe { OpenClipboard(0 as HWND) };
        if opened == 0 {
            return Err(clipboard_error(context));
        }

        Ok(Self)
    }
}

#[cfg(target_os = "windows")]
impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = windows_sys::Win32::System::DataExchange::CloseClipboard();
        }
    }
}

#[cfg(target_os = "windows")]
fn clipboard_error(context: &'static str) -> TextInsertionError {
    TextInsertionError::Clipboard {
        context,
        source: std::io::Error::last_os_error(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_is_a_successful_noop() {
        let result = TextInsertionService
            .insert("", &TextInsertionSettings::default())
            .unwrap();

        assert!(result.success);
        assert_eq!(result.inserted_characters, 0);
    }
}
