---
type: ADR
id: "0173"
title: "Opt-in tray resident mode"
status: active
date: 2026-07-31
---

## Context

Closing Tolaria's main window quits the app. On macOS that is unusual for a
knowledge base people keep open all day: reopening pays the full startup path
described in [ADR-0166](0166-snapshot-first-progressive-vault-startup.md), and
the Git background work owned by the main window
([ADR-0165](0165-window-owned-vault-watchers-and-main-window-git-background-work.md))
stops entirely. Users have asked for the app to stay resident behind a menu bar
icon instead.

Making that the default would change close behavior for every existing user,
and a resident app that leaves no visible affordance is worse than one that
quits. Tolaria also already has three window kinds — `main`, `note-*` and
`ai-workspace` — so "the app has no windows" is not the same question as "the
main window is closed".

Tauri's `tray-icon` feature is not currently enabled, so this is a new
dependency surface as well as a new platform behavior.

## Decision

**Tolaria gains an opt-in `tray_resident_mode_enabled` app setting, default
off. While it is on, closing the main window hides it behind a menu bar /
system tray icon instead of quitting; all decisions about when to intercept,
when to change activation policy, and when to create or destroy the icon live
in a pure `tray_state` module, with `tray.rs` as thin Tauri glue.**

The preference is device-specific, so per
[ADR-0004](0004-vault-vs-app-settings-storage.md) it belongs in app settings
rather than the vault.

## Options considered

- **Opt-in setting with a Rust-owned tray** (chosen): existing users see no
  change, the tray appears and disappears without a restart, and the renderer
  only reports the preference through one `set_tray_resident_mode` command — so
  no tray capability has to be added to `capabilities/default.json`. Cons: one
  more branch through the close path, and a `quitting` flag has to exist so the
  real exit path is not turned back into a hide.
- **Always resident**: simpler, no setting, no flag. Rejected — it silently
  changes what the close button means for everyone, and on Linux the tray
  depends on `libayatana-appindicator3`, which is not guaranteed to be present.
- **Tray-anchored popover or mini window**: a second, smaller surface hanging
  off the icon. Rejected — Tolaria's main window is the window
  ([ADR-0031](0031-full-app-for-note-windows.md)), and a reduced surface would
  diverge from it exactly the way `NoteWindow` did.
- **Renderer-driven tray**: expose tray create/destroy/menu APIs to the
  frontend. Rejected — it needs extra capabilities, and window lifecycle
  already lives in Rust next to `window_state`.

## Consequences

- Existing installs keep quitting on close; the behavior only changes after a
  deliberate opt-in.
- Close interception runs in `handle_run_event` after `window_state` has
  persisted the frame, so hiding does not regress window restore. This depends
  on Tauri delivering `CloseRequested` to both the per-window listeners and the
  global run-event callback before it reads the close signal; a future Tauri
  change to that ordering would need re-verification.
- Only the `main` label is intercepted, so `note-*` and `ai-workspace` windows
  are unaffected.
- On macOS the app switches to `ActivationPolicy::Accessory` only when no
  window is visible, so an open note window is never orphaned, and back to
  `Regular` whenever the window is shown.
- Enabling `tray-icon` pulls the `tray_icon` crate into every desktop build,
  and Linux packaging now genuinely needs `libayatana-appindicator3`. Tray
  creation failures log and continue rather than aborting startup.
- Linux never emits tray icon click events, so the tray menu is the only
  interaction there; the left-click-to-restore path is macOS and Windows only.
- Tray menu labels are native strings and are not localized, matching the
  existing native menu ([ADR-0052](0052-renderer-first-shortcut-execution-with-native-menu-dedupe.md));
  all renderer copy for the setting is localized normally.
- Triggers re-evaluation if: resident mode becomes the default, a global
  show/hide shortcut is added (which needs its own shortcut-conflict design on
  top of [ADR-0050](0050-deterministic-shortcut-command-routing.md)), or launch
  at login is introduced.
