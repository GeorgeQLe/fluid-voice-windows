# History

## 2026-07-01

- Created the initial Windows-native FluidVoice MVP scaffold under `apps/windows`.
- Added Tauri v2 desktop configuration, React/TypeScript UI, Rust service modules, model download/catalog support, settings/history stores, and Windows insertion boundaries.
- Updated Vite to 6.4.3 after npm audit surfaced Vite/esbuild advisories.
- Verified TypeScript typecheck, frontend production build, npm audit, and Cargo manifest resolution.
- Rust desktop test build remains blocked on this Linux host by missing `pkg-config`/DBus development dependencies.
