# Derek Msg Sync

Windows desktop receiver for iPhone SMS verification codes.

The app runs a local LAN HTTP endpoint, accepts messages sent from iPhone Shortcuts, extracts verification codes, copies the latest code to the Windows clipboard, and keeps a local message history.

## Features

- Tauri 2 desktop app with Vue 3 UI.
- LAN receiver endpoint for iPhone Shortcuts.
- Automatic verification code extraction and clipboard copy.
- Compact message inbox with local persistence.
- Tray icon support: close hides the window, tray click restores it, tray menu can exit.
- Custom in-app notification window, with a settings toggle.
- Configurable receiver port and optional local token.
- Optional Windows startup launch.

## Development

```powershell
npm install
npm run tauri dev
```

## Build

```powershell
npm run tauri build -- --bundles nsis
```

## License

Apache-2.0
