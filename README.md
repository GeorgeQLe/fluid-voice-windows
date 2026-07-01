# FluidVoice Windows

Windows-native MVP for FluidVoice using Tauri v2, Rust, React, and TypeScript.

The initial scope is core dictation:

- Tray-hosted Windows desktop app.
- Settings, model cache, history, and credential storage boundaries.
- WASAPI/cpal microphone capture with 16 kHz mono PCM conversion.
- Whisper.cpp-compatible model catalog and cache management.
- Optional OpenAI-compatible transcript enhancement.
- Windows text insertion boundary using `SendInput` with a clipboard fallback path reserved.

## Layout

```text
apps/windows/              React and Tauri Windows app
apps/windows/src/          TypeScript UI
apps/windows/src-tauri/    Rust desktop core
```

## Development

```bash
npm install
npm run windows:dev
```

## Verification

```bash
npm run windows:test
cargo test -p fluid-voice-windows
```

Windows-specific hotkeys, text insertion, installer behavior, toast notifications, and microphone permission flows still require a Windows 11 acceptance pass.

