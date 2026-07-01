use crate::settings::{HotkeyMode, HotkeySettings};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
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

#[derive(Debug, Error)]
pub enum HotkeyError {
    #[error("shortcut must include at least one modifier and one key")]
    InvalidShortcut,
    #[error("hotkey state lock was poisoned")]
    LockPoisoned,
}

#[derive(Default)]
pub struct HotkeyService {
    current: Mutex<Option<HotkeySettings>>,
}

impl HotkeyService {
    pub fn apply(&self, settings: &HotkeySettings) -> Result<HotkeyRegistration, HotkeyError> {
        validate_shortcut(&settings.shortcut)?;

        let mut current = self.current.lock().map_err(|_| HotkeyError::LockPoisoned)?;
        *current = Some(settings.clone());

        let status = if !settings.enabled {
            HotkeyRegistrationStatus::Disabled
        } else if cfg!(target_os = "windows") {
            HotkeyRegistrationStatus::Ready
        } else {
            HotkeyRegistrationStatus::RequiresWindowsMessageLoop
        };

        Ok(HotkeyRegistration {
            enabled: settings.enabled,
            shortcut: settings.shortcut.clone(),
            mode: settings.mode.clone(),
            status,
        })
    }

    pub fn current(&self) -> Result<Option<HotkeyRegistration>, HotkeyError> {
        let settings = self
            .current
            .lock()
            .map_err(|_| HotkeyError::LockPoisoned)?
            .clone();
        settings
            .as_ref()
            .map(|settings| self.apply(settings))
            .transpose()
    }
}

fn validate_shortcut(shortcut: &str) -> Result<(), HotkeyError> {
    let parts = shortcut
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let has_modifier = parts.iter().any(|part| {
        matches!(
            part.to_ascii_lowercase().as_str(),
            "ctrl" | "control" | "alt" | "shift" | "win" | "meta"
        )
    });

    if parts.len() < 2 || !has_modifier {
        return Err(HotkeyError::InvalidShortcut);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_shortcut_without_modifier() {
        assert!(validate_shortcut("Space").is_err());
    }

    #[test]
    fn accepts_modifier_plus_key() {
        assert!(validate_shortcut("Ctrl+Alt+Space").is_ok());
    }
}
