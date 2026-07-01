# Todo

## Current

- [ ] Run a Windows 11 acceptance pass for the Tauri app: install/build, microphone capture, model download, Whisper transcription, SendInput insertion, tray quit, and installer behavior.
- [ ] Validate Windows-native integration behavior on Windows 11: `RegisterHotKey` delivery, clipboard paste fallback preservation, focused-app insertion reliability, and tray quit cleanup.
- [ ] Implement post-MVP Windows niceties: startup-at-login and toast notifications.
- [ ] Add CI or a Windows build job once the first Windows acceptance pass succeeds.

## Completed

- [x] Scaffold Windows-native FluidVoice MVP using Tauri v2, Rust, React, and TypeScript.
- [x] Add core service boundaries for settings, history, model cache, microphone capture, Whisper transcription, enhancement, secrets, hotkeys, and text insertion.
- [x] Add main settings UI, dictation controls, model management, enhancement settings, history view, overlay window, and Tauri tray shell.
- [x] Add `RegisterHotKey`-based toggle event wiring, clipboard paste fallback with text restoration, model download cancellation, and insertion settings UI.
- [x] Add the private dogfood MVP PRD, acceptance criteria, known risks, and roadmap.
- [x] Add repository README and package/Cargo workspace metadata.

## Blockers

- Local Linux Rust validation cannot complete until the host has the desktop development dependencies required by Tauri's Linux stack, specifically `pkg-config` and DBus development headers.
- Windows-specific behavior still needs a real Windows 11 validation pass.
