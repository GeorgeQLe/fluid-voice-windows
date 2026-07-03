# FluidVoice Windows

Windows-native MVP for FluidVoice using Tauri v2, Rust, React, and TypeScript.

The initial scope is core dictation:

- Taskbar-visible Windows desktop app with tray controls.
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

The main FluidVoice window is configured to appear in the Windows taskbar. The compact recording overlay stays off the taskbar.

## Windows packaging

Run these commands from a Windows development shell:

```bash
npm run windows:exe
```

This creates the bare app executable at `target/release/fluid-voice-windows.exe`.

```bash
npm run windows:installer
```

This creates an NSIS installer executable under `target/release/bundle/nsis/`.

```bash
npm run windows:build
```

This creates the configured Windows bundles, currently NSIS and MSI, under `target/release/bundle/`.

## Verification

```bash
npm run windows:test
npm --workspace apps/windows run build
npm audit --json
cargo test -p fluid-voice-windows
```

Windows-specific installer behavior, microphone permission flows, hotkey delivery, and insertion reliability still require a Windows 11 acceptance pass.
