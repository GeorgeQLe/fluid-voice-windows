# History

## 2026-07-01

- Created the initial Windows-native FluidVoice MVP scaffold under `apps/windows`.
- Added Tauri v2 desktop configuration, React/TypeScript UI, Rust service modules, model download/catalog support, settings/history stores, and Windows insertion boundaries.
- Updated Vite to 6.4.3 after npm audit surfaced Vite/esbuild advisories.
- Verified TypeScript typecheck, frontend production build, npm audit, and Cargo manifest resolution.
- Rust desktop test build remains blocked on this Linux host by missing `pkg-config`/DBus development dependencies.
- Added the private dogfood MVP PRD and roadmap under `docs/windows-mvp-prd.md`.
- Implemented Windows hotkey event registration scaffolding through `RegisterHotKey`, frontend hotkey toggle handling, clipboard paste insertion fallback with text clipboard restoration, model download cancellation, and insertion settings UI.
- Re-verified TypeScript typecheck, frontend production build, npm audit, and Rust formatting. Rust tests remain blocked on this Linux host by missing `pkg-config`/DBus development dependencies.

## 2026-07-03

- Made the main FluidVoice Tauri window explicitly taskbar-visible while keeping the compact recording overlay off the taskbar.
- Added root scripts for producing a bare Windows executable and an NSIS installer, and documented the release artifact paths.
- Verified TypeScript typecheck and frontend production build. Full Tauri dev/build and Rust tests remain blocked on this Linux host by missing `pkg-config`, DBus/ALSA development headers, and `libclang`; Windows executable/installer generation still needs a Windows development shell.
