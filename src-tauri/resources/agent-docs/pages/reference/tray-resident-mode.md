# Background Mode

Source: reference/tray-resident-mode.md
URL: /reference/tray-resident-mode

# Background Mode

By default, closing the Tolaria window quits the app. Background mode changes that: Tolaria keeps running behind an icon in the macOS menu bar, or in the Windows and Linux system tray, until you quit it explicitly.

The setting is off by default. Nothing changes until you turn it on.

## Turning It On

Open **Settings → Desktop** and enable **Keep Tolaria running in the background**.

The icon appears immediately. You do not need to restart the app.

## What Changes

| Action | Background mode off | Background mode on |
| --- | --- | --- |
| Close the main window | Tolaria quits | The window hides and Tolaria keeps running |
| Click the icon | — | The window comes back where you left it |
| Right-click the icon | — | Opens a menu with **Show Tolaria** and **Quit Tolaria** |
| `Cmd+Q` / **Quit Tolaria** | Tolaria quits | Tolaria quits |

Note windows and the detached AI workspace window are unaffected. Closing one of those always closes just that window, in either mode.

On macOS, Tolaria leaves the Dock and the `Cmd+Tab` switcher while it is hidden, and returns to both when you bring the window back. If a note window is still open, Tolaria stays in the Dock so that window remains reachable.

## Quitting

Background mode never traps a quit. Use whichever you prefer:

- **Quit Tolaria** from the icon's menu
- `Cmd+Q` on macOS, or **Quit** from the app menu

Launching Tolaria again while it is hidden brings the running app back instead of starting a second copy.

## Turning It Off

Turn the setting off in **Settings → Desktop**. The icon disappears, and closing the window quits the app again. If the window was hidden at the time, Tolaria brings it back so you are never left without a way into the app.

## Linux

The tray icon needs `libayatana-appindicator3`, which is part of the Linux dependency list in the project README. If it is missing, Tolaria logs the problem and starts normally without the icon — close still quits.

Linux desktops do not report clicks on tray icons, so use the icon's menu to show or quit Tolaria there.