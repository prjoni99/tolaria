//! Tauri glue for tray resident mode.
//!
//! Every decision lives in [`crate::tray_state`]; this module only translates
//! those decisions into Tauri calls. Native tray menu labels stay in English
//! here for the same reason the native app menu does — Tolaria's native menus
//! are not routed through the renderer locale bundle.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::menu::{Menu, MenuBuilder, MenuEvent, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager};

use crate::tray_state::{tray_should_exist, TrayAction};
use crate::window_state::MAIN_WINDOW_LABEL;

const TRAY_ID: &str = "tolaria-resident-tray";
const TRAY_TOOLTIP: &str = "Tolaria";
const SHOW_MENU_ITEM_ID: &str = "tray-show-tolaria";
const SHOW_MENU_ITEM_LABEL: &str = "Show Tolaria";
const QUIT_MENU_ITEM_ID: &str = "tray-quit-tolaria";
const QUIT_MENU_ITEM_LABEL: &str = "Quit Tolaria";
const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/128x128@2x.png");

/// Resident-mode flags shared between the tray handlers and the
/// `set_tray_resident_mode` command.
#[derive(Debug, Default)]
pub(crate) struct TrayResidentState {
    enabled: AtomicBool,
}

/// Mount the tray for the persisted preference. A tray failure must never stop
/// the app from starting, so problems are logged and startup continues.
pub(crate) fn setup(app: &App) {
    let app_handle = app.handle();
    app_handle.on_menu_event(handle_tray_menu_event);

    let enabled = crate::settings::tray_resident_mode_enabled();
    set_enabled(app_handle, enabled);
    if let Err(error) = sync_tray_icon(app_handle, enabled) {
        log::warn!("Failed to set up the tray icon: {error}");
    }
}

/// Apply a preference change without an app restart.
pub(crate) fn set_resident_mode(app_handle: &AppHandle, enabled: bool) -> Result<(), String> {
    set_enabled(app_handle, enabled);
    sync_tray_icon(app_handle, enabled).map_err(|error| format!("Tray update failed: {error}"))
}

/// Bring the main window back to the front.
pub(crate) fn show_main_window(app_handle: &AppHandle) {
    let Some(window) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

fn sync_tray_icon(app_handle: &AppHandle, enabled: bool) -> tauri::Result<()> {
    match tray_should_exist(enabled, app_handle.tray_by_id(TRAY_ID).is_some()) {
        TrayAction::Create => create_tray_icon(app_handle),
        TrayAction::Destroy => {
            app_handle.remove_tray_by_id(TRAY_ID);
            Ok(())
        }
        TrayAction::None => Ok(()),
    }
}

fn create_tray_icon(app_handle: &AppHandle) -> tauri::Result<()> {
    let menu = build_tray_menu(app_handle)?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tauri::image::Image::from_bytes(TRAY_ICON_BYTES)?)
        .icon_as_template(true)
        .tooltip(TRAY_TOOLTIP)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(handle_tray_icon_event)
        .build(app_handle)?;
    Ok(())
}

fn build_tray_menu(app_handle: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItemBuilder::with_id(SHOW_MENU_ITEM_ID, SHOW_MENU_ITEM_LABEL).build(app_handle)?;
    let quit = MenuItemBuilder::with_id(QUIT_MENU_ITEM_ID, QUIT_MENU_ITEM_LABEL).build(app_handle)?;
    MenuBuilder::new(app_handle)
        .item(&show)
        .separator()
        .item(&quit)
        .build()
}

fn handle_tray_menu_event(app_handle: &AppHandle, event: MenuEvent) {
    match event.id().0.as_str() {
        SHOW_MENU_ITEM_ID => show_main_window(app_handle),
        QUIT_MENU_ITEM_ID => app_handle.exit(0),
        _ => {}
    }
}

fn handle_tray_icon_event(tray: &TrayIcon, event: TrayIconEvent) {
    if is_primary_activation(&event) {
        show_main_window(tray.app_handle());
    }
}

/// A finished left click. Right clicks open the menu, and Linux never emits
/// tray icon events at all.
fn is_primary_activation(event: &TrayIconEvent) -> bool {
    matches!(
        event,
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        }
    )
}

fn resident_state(app_handle: &AppHandle) -> tauri::State<'_, TrayResidentState> {
    app_handle.state()
}

fn set_enabled(app_handle: &AppHandle, enabled: bool) {
    resident_state(app_handle)
        .enabled
        .store(enabled, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn click_event(button: MouseButton, button_state: MouseButtonState) -> TrayIconEvent {
        TrayIconEvent::Click {
            id: TRAY_ID.into(),
            position: tauri::PhysicalPosition::new(0.0, 0.0),
            rect: tauri::Rect {
                position: tauri::PhysicalPosition::new(0.0, 0.0).into(),
                size: tauri::PhysicalSize::new(0.0, 0.0).into(),
            },
            button,
            button_state,
        }
    }

    #[test]
    fn only_a_finished_left_click_reopens_the_window() {
        assert!(is_primary_activation(&click_event(
            MouseButton::Left,
            MouseButtonState::Up
        )));
        assert!(!is_primary_activation(&click_event(
            MouseButton::Left,
            MouseButtonState::Down
        )));
        assert!(!is_primary_activation(&click_event(
            MouseButton::Right,
            MouseButtonState::Up
        )));
        assert!(!is_primary_activation(&click_event(
            MouseButton::Middle,
            MouseButtonState::Up
        )));
    }
}
