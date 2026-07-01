# FluidVoice Windows

Windows-native MVP for FluidVoice using Tauri v2, Rust, React, and TypeScript.

The initial scope is core dictation:

- Tray-hosted Windows desktop app.
- Settings, model cache, history, and credential storage boundaries.
- WASAPI/cpal microphone capture with 16 kHz mono PCM conversion.
- Whisper.cpp-compatible model catalog and cache management.
- Optional OpenAI-compatible transcript enhancement.
- Windows text insertion using `SendInput` with clipboard paste fallback.

See [docs/windows-mvp-prd.md](docs/windows-mvp-prd.md) for the private dogfood MVP acceptance criteria, non-goals, known risks, and roadmap.

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
npm --workspace apps/windows run build
npm audit --json
cargo test -p fluid-voice-windows
```

Windows-specific installer behavior, microphone permission flows, hotkey delivery, and insertion reliability still require a Windows 11 acceptance pass.
