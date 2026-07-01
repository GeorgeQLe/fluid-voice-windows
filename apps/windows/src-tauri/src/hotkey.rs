use crate::settings::{HotkeyMode, HotkeySettings};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::AppHandle;
#[cfg(target_os = "windows")]
use tauri::Emitter;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyRegistration {
    pub enabled: bool,
    pub shortcut: String,
    pub mode: HotkeyMode,
    pub status: HotkeyRegistrationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyRegistrationStatus {
    Ready,
    Disabled,
    RequiresWindowsMessageLoop,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyEvent {
    pub shortcut: String,
    pub mode: HotkeyMode,
}

#[derive(Debug, Error)]
pub enum HotkeyError {
    #[error("shortcut must include at least one modifier and exactly one key")]
    InvalidShortcut,
    #[error("only toggle hotkey mode is supported in the Windows MVP")]
    UnsupportedMode,
    #[error("hotkey events require a Tauri app handle")]
    MissingAppHandle,
    #[error("failed to register global hotkey: {0}")]
    RegistrationFailed(String),
    #[error("hotkey state lock was poisoned")]
    LockPoisoned,
}

#[derive(Default)]
pub struct HotkeyService {
    app_handle: Option<AppHandle>,
    state: Mutex<HotkeyState>,
}

#[derive(Default)]
struct HotkeyState {
    settings: Option<HotkeySettings>,
    status: Option<HotkeyRegistrationStatus>,
    registration: Option<PlatformHotkey>,
}

impl HotkeyService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle: Some(app_handle),
            state: Mutex::new(HotkeyState::default()),
        }
    }

    pub fn apply(&self, settings: &HotkeySettings) -> Result<HotkeyRegistration, HotkeyError> {
        validate_shortcut(&settings.shortcut)?;
        if settings.mode != HotkeyMode::Toggle {
            return Err(HotkeyError::UnsupportedMode);
        }

        {
            let state = self.state.lock().map_err(|_| HotkeyError::LockPoisoned)?;
            if state.settings.as_ref() == Some(settings) {
                return Ok(registration_for(
                    settings,
                    state
                        .status
                        .clone()
                        .unwrap_or(HotkeyRegistrationStatus::Disabled),
                ));
            }
        }

        let (next_registration, next_status) = if settings.enabled {
            register_platform_hotkey(settings, self.app_handle.clone())?
        } else {
            (None, HotkeyRegistrationStatus::Disabled)
        };

        let previous = {
            let mut state = self.state.lock().map_err(|_| HotkeyError::LockPoisoned)?;
            let previous = state.registration.take();
            state.settings = Some(settings.clone());
            state.status = Some(next_status.clone());
            state.registration = next_registration;
            previous
        };
        drop(previous);

        Ok(registration_for(settings, next_status))
    }

    pub fn current(&self) -> Result<Option<HotkeyRegistration>, HotkeyError> {
        let state = self.state.lock().map_err(|_| HotkeyError::LockPoisoned)?;
        Ok(state.settings.as_ref().map(|settings| {
            registration_for(
                settings,
                state
                    .status
                    .clone()
                    .unwrap_or(HotkeyRegistrationStatus::Disabled),
            )
        }))
    }
}

fn registration_for(
    settings: &HotkeySettings,
    status: HotkeyRegistrationStatus,
) -> HotkeyRegistration {
    HotkeyRegistration {
        enabled: settings.enabled,
        shortcut: settings.shortcut.clone(),
        mode: settings.mode.clone(),
        status,
    }
}

fn validate_shortcut(shortcut: &str) -> Result<(), HotkeyError> {
    shortcut_parts(shortcut).map(|_| ())
}

struct ShortcutParts {
    modifiers: Vec<String>,
    key: String,
}

fn shortcut_parts(shortcut: &str) -> Result<ShortcutParts, HotkeyError> {
    let parts = shortcut
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    let mut modifiers = Vec::new();
    let mut keys = Vec::new();

    for part in parts {
        if is_modifier(part) {
            modifiers.push(part.to_ascii_lowercase());
        } else {
            keys.push(part.to_string());
        }
    }

    if modifiers.is_empty() || keys.len() != 1 {
        return Err(HotkeyError::InvalidShortcut);
    }

    Ok(ShortcutParts {
        modifiers,
        key: keys.remove(0),
    })
}

fn is_modifier(part: &str) -> bool {
    matches!(
        part.to_ascii_lowercase().as_str(),
        "ctrl" | "control" | "alt" | "shift" | "win" | "meta"
    )
}

#[cfg(target_os = "windows")]
struct PlatformHotkey {
    thread_id: u32,
    join: Option<std::thread::JoinHandle<()>>,
}

#[cfg(not(target_os = "windows"))]
struct PlatformHotkey;

#[cfg(target_os = "windows")]
fn register_platform_hotkey(
    settings: &HotkeySettings,
    app_handle: Option<AppHandle>,
) -> Result<(Option<PlatformHotkey>, HotkeyRegistrationStatus), HotkeyError> {
    let app_handle = app_handle.ok_or(HotkeyError::MissingAppHandle)?;
    Ok((
        Some(PlatformHotkey::register(settings, app_handle)?),
        HotkeyRegistrationStatus::Ready,
    ))
}

#[cfg(not(target_os = "windows"))]
fn register_platform_hotkey(
    _settings: &HotkeySettings,
    _app_handle: Option<AppHandle>,
) -> Result<(Option<PlatformHotkey>, HotkeyRegistrationStatus), HotkeyError> {
    Ok((None, HotkeyRegistrationStatus::RequiresWindowsMessageLoop))
}

#[cfg(target_os = "windows")]
impl PlatformHotkey {
    fn register(settings: &HotkeySettings, app_handle: AppHandle) -> Result<Self, HotkeyError> {
        use std::sync::mpsc;
        use windows_sys::Win32::{
            Foundation::HWND,
            System::Threading::GetCurrentThreadId,
            UI::WindowsAndMessaging::{
                GetMessageW, PeekMessageW, RegisterHotKey, UnregisterHotKey, MSG, PM_NOREMOVE,
                WM_HOTKEY, WM_QUIT, WM_USER,
            },
        };

        const HOTKEY_ID: i32 = 0x4656;

        let shortcut = parse_windows_shortcut(&settings.shortcut)?;
        let event = HotkeyEvent {
            shortcut: settings.shortcut.clone(),
            mode: settings.mode.clone(),
        };
        let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();

        let join = std::thread::Builder::new()
            .name("fluidvoice-hotkey".to_string())
            .spawn(move || unsafe {
                let thread_id = GetCurrentThreadId();
                let hwnd = 0 as HWND;
                let registered = RegisterHotKey(hwnd, HOTKEY_ID, shortcut.modifiers, shortcut.vk);
                if registered == 0 {
                    let _ = ready_tx.send(Err(std::io::Error::last_os_error().to_string()));
                    return;
                }

                let mut message = std::mem::zeroed::<MSG>();
                let _ = PeekMessageW(&mut message, hwnd, WM_USER, WM_USER, PM_NOREMOVE);
                let _ = ready_tx.send(Ok(thread_id));

                loop {
                    let result = GetMessageW(&mut message, hwnd, 0, 0);
                    if result <= 0 || message.message == WM_QUIT {
                        break;
                    }

                    if message.message == WM_HOTKEY && message.wParam == HOTKEY_ID as usize {
                        let _ = app_handle.emit("dictation-hotkey-toggle", event.clone());
                    }
                }

                let _ = UnregisterHotKey(hwnd, HOTKEY_ID);
            })
            .map_err(|error| HotkeyError::RegistrationFailed(error.to_string()))?;

        match ready_rx
            .recv()
            .map_err(|error| HotkeyError::RegistrationFailed(error.to_string()))?
        {
            Ok(thread_id) => Ok(Self {
                thread_id,
                join: Some(join),
            }),
            Err(error) => {
                let _ = join.join();
                Err(HotkeyError::RegistrationFailed(error))
            }
        }
    }

    fn stop(&mut self) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};

        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
        }

        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for PlatformHotkey {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
struct WindowsShortcut {
    modifiers: u32,
    vk: u32,
}

#[cfg(target_os = "windows")]
fn parse_windows_shortcut(shortcut: &str) -> Result<WindowsShortcut, HotkeyError> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
    };

    let parts = shortcut_parts(shortcut)?;
    let mut modifiers = MOD_NOREPEAT;

    for modifier in parts.modifiers {
        match modifier.as_str() {
            "ctrl" | "control" => modifiers |= MOD_CONTROL,
            "alt" => modifiers |= MOD_ALT,
            "shift" => modifiers |= MOD_SHIFT,
            "win" | "meta" => modifiers |= MOD_WIN,
            _ => return Err(HotkeyError::InvalidShortcut),
        }
    }

    Ok(WindowsShortcut {
        modifiers,
        vk: virtual_key(&parts.key).ok_or(HotkeyError::InvalidShortcut)?,
    })
}

#[cfg(target_os = "windows")]
fn virtual_key(key: &str) -> Option<u32> {
    let normalized = key.trim().to_ascii_lowercase();

    if normalized.len() == 1 {
        let byte = normalized.as_bytes()[0];
        if byte.is_ascii_alphanumeric() {
            return Some(byte.to_ascii_uppercase() as u32);
        }
    }

    if let Some(number) = normalized
        .strip_prefix('f')
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|number| (1..=24).contains(number))
    {
        return Some(0x70 + number - 1);
    }

    match normalized.as_str() {
        "space" => Some(0x20),
        "enter" | "return" => Some(0x0D),
        "tab" => Some(0x09),
        "esc" | "escape" => Some(0x1B),
        "backspace" => Some(0x08),
        "delete" | "del" => Some(0x2E),
        "insert" | "ins" => Some(0x2D),
        "home" => Some(0x24),
        "end" => Some(0x23),
        "pageup" | "page up" => Some(0x21),
        "pagedown" | "page down" => Some(0x22),
        "left" | "arrowleft" => Some(0x25),
        "up" | "arrowup" => Some(0x26),
        "right" | "arrowright" => Some(0x27),
        "down" | "arrowdown" => Some(0x28),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_shortcut_without_modifier() {
        assert!(validate_shortcut("Space").is_err());
    }

    #[test]
    fn rejects_shortcut_without_key() {
        assert!(validate_shortcut("Ctrl+Alt").is_err());
    }

    #[test]
    fn accepts_modifier_plus_key() {
        assert!(validate_shortcut("Ctrl+Alt+Space").is_ok());
    }
}
