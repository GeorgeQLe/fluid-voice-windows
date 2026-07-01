# FluidVoice Windows Private Dogfood MVP PRD

## Summary

FluidVoice Windows is a private dogfood MVP for proving the Windows 11 dictation loop before pursuing parity with upstream macOS FluidVoice. The success bar is internal usability: a trusted tester can run the app, download a Whisper model, trigger dictation from another app, record speech, transcribe locally, optionally enhance the text through an OpenAI-compatible provider, and insert the final text into the focused text field.

Primary audience: the project owner and a small trusted Windows 11 test group.

Primary goal: validate the Windows technical foundation and end-to-end dictation loop before adding upstream parity features.

## MVP Scope

- Windows desktop tray app using Tauri v2, Rust, React, and TypeScript.
- Tray show/hide/quit behavior.
- Input microphone selection, including Windows default input.
- Local Whisper model catalog, download, cancel, clear, and re-download flows.
- Toggle-only global hotkey using Windows `RegisterHotKey`.
- WASAPI/cpal recording converted to 16 kHz mono PCM/f32 for transcription.
- Local Whisper/whisper.cpp-compatible ggml transcription.
- Always-on-top compact overlay with recording duration and audio level.
- Final text insertion through `SendInput`, with clipboard paste fallback and prior text clipboard restoration when possible.
- Versioned JSON settings and local JSON history.
- API keys stored in Windows Credential Manager through `keyring`.
- Optional OpenAI-compatible chat completion enhancement for OpenAI, Groq, and custom endpoints.

## Required Settings

- Input device.
- Language.
- Selected Whisper model.
- Hotkey enabled/shortcut.
- Enhancement enabled/provider/base URL/model/prompt profile.
- History enabled/max items.
- Insertion mode and clipboard restoration.
- Startup-at-login placeholder for post-MVP implementation.

## Non-Goals

- Command Mode.
- Rewrite or selected-text editing mode.
- Fluid Intelligence local AI runtime.
- Parakeet, Nemotron, Cohere, Apple Speech, or Apple Speech Analyzer support.
- Per-app prompt routing.
- Custom dictionary import/export.
- Meeting transcription.
- Local API server.
- Analytics, feedback reporting, or public telemetry.
- Auto-updater, beta channel, or public release process.
- Full upstream macOS feature parity.

## Acceptance Criteria

- Fresh dev run or install starts without manual file edits.
- Tray icon can show, hide, and quit the app.
- Settings window loads without runtime permission errors.
- User can download Whisper Base English from the UI.
- User can select a microphone.
- User can set a global shortcut.
- From Notepad, VS Code, Chrome, Word, and Slack or Teams, the user can focus a text field, press the hotkey, speak for 3-15 seconds, press the hotkey again, and receive inserted text in the original target field.
- Raw transcription works with enhancement disabled.
- Enhancement works with at least one OpenAI-compatible provider when an API key is configured.
- Enhancement failure falls back to the raw transcript.
- Clipboard fallback restores prior text clipboard content when possible.
- Canceling dictation does not insert text or create a misleading success history entry.
- Quitting releases microphone stream and hotkey registration.
- History records successful dictations.
- Secrets are not written to settings, history, or logs.

## Automated Verification

- `npm run windows:test`
- `npm --workspace apps/windows run build`
- `npm audit --json`
- `cargo test -p fluid-voice-windows` on a host with Tauri desktop system dependencies
- Windows build/test job once CI is added

Unit coverage should include settings migration/defaults, model catalog/cache validation, history retention, hotkey shortcut parsing, prompt rendering/request body construction, and clipboard fallback restoration where practical.

## Manual Dogfood Pass

- Windows 11 fresh machine or VM.
- Install or run the app.
- Download Whisper Base English.
- Record and insert into Notepad, VS Code, Chrome, Microsoft Word, and Slack or Teams.
- Test hotkey start/stop, UI start/stop, cancel, missing model error, microphone unavailable error, API key save/delete, enhancement success/failure, clipboard preservation, and tray quit cleanup.

## Known MVP Risks

- Model checksum validation is not implemented. The app detects missing and zero-byte model files, cleans canceled partial downloads, and refuses transcription when the selected model is not downloaded, but corrupt non-empty model files may still reach Whisper and fail there.
- Windows-specific hotkey, clipboard, insertion, installer, and microphone behavior still require a Windows 11 acceptance pass.
- Clipboard fallback restores text clipboard content only. Arbitrary binary or multi-format clipboard content is not preserved in the MVP.

## Roadmap

### Phase 1: Stabilize Dogfood

- Fix Windows 11 acceptance failures.
- Add Windows CI build and test job.
- Add structured logging and local diagnostic export.
- Harden model cache validation, incomplete download cleanup, and cancellation.
- Improve insertion reliability across common Windows apps.
- Add startup-at-login.
- Add Windows toast notifications for hotkey, microphone, model, and insertion failures.
- Validate installer behavior on a clean Windows VM.

### Phase 2: Public Alpha Readiness

- Polish onboarding for microphone selection, first model download, hotkey setup, first dictation, and optional enhancement setup.
- Add support docs and troubleshooting for Windows permissions, microphone errors, hotkeys, and insertion.
- Add opt-in crash/error reporting only if explicitly chosen.
- Add a signed installer plan, release checklist, public privacy note, icon/assets, and polished tray/menu states.

### Phase 3: Feature Parity Candidates

- Hold-mode hotkey, multiple hotkeys, mouse shortcuts, and shortcut conflict detection.
- Custom dictionary with JSON import/export.
- Prompt profiles and per-app prompt routing.
- Rewrite Mode and Command Mode.
- Rich history and stats.
- Optional audio history with retention budget and export.
- Better live preview or streaming transcription.
- Additional Whisper model sizes.
- Local API server.
- Meeting transcription.
- Auto-updater and beta channel.

### Phase 4: Advanced Model And Local AI Work

- Evaluate Windows-compatible model backends for Parakeet, Nemotron, and Cohere equivalents.
- Add GPU acceleration if practical.
- Add local non-cloud enhancement only after core dictation is stable.
- Avoid direct CoreML or Apple Speech parity on Windows; use Windows-compatible runtimes only.
- Treat upstream Fluid Intelligence as out of scope until a Windows-compatible licensed/runtime path exists.
