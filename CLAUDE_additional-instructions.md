# Supercharged Fork Context

This is the `supercharged` branch — a personal fork of [Handy](https://github.com/cjpais/Handy) maintained by @ahoendgen.

## Purpose

This is not a standalone project. The goal is to have a personal, customized variant of Handy that allows quickly implementing features needed for specific workflows without waiting for upstream releases. Changes that make sense for the broader community may be contributed back to the original project.

## Key Differences from Upstream

- **Product Name**: "Handy Supercharged" (space, no hyphen — a hyphen in `productName` breaks the macOS tray icon)
- **Identifier**: `com.a9g.handy-supercharged` (separate from upstream so both can coexist)
- **Color Theme**: Electric blue (`#3399ff`) instead of pink (`#FAA2CA`)
- **Tray Icon**: Lightning bolt mascot instead of hand icon
- **Updater**: Removed — this fork does not use the upstream update checker
- **Trigger Words**: Custom feature for executing actions via spoken keywords during transcription

## Build Notes

- macOS signing uses `APPLE_SIGNING_IDENTITY` from a local `.envrc` file (not committed)
- Always use `CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri build` for production builds
- The `productName` must NOT contain a hyphen — this causes the tray icon to silently fail on macOS
