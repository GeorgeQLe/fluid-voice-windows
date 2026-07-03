# Lessons

## 2026-07-03

- When the user asks about the executable path, distinguish the release executable from `tauri dev`: development mode starts the Vite dev server, while `target/release/fluid-voice-windows.exe` should use built frontend assets and must not bind a dev server.
