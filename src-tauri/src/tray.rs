//! Tauri glue for tray resident mode.
//!
//! Every decision lives in [`crate::tray_state`]; this module only translates
//! those decisions into Tauri calls. Native tray menu labels stay in English
//! here for the same reason the native app menu does — Tolaria's native menus
//! are not routed through the renderer locale bundle.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::menu::{Menu, MenuBuilder, MenuEvent, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, CloseRequestApi, Manager, RunEvent, WindowEvent};

use crate::tray_state::{
    should_intercept_close, should_restore_hidden_window, should_use_accessory_policy,
    tray_should_exist, TrayAction,
};
use crate::window_state::MAIN_WINDOW_LABEL;

const TRAY_ID: &str = "tolaria-resident-tray";
const TRAY_TOOLTIP: &str = "Tolaria";
const SHOW_MENU_ITEM_ID: &str = "tray-show-tolaria";
const SHOW_MENU_ITEM_LABEL: &str = "Show Tolaria";
const QUIT_MENU_ITEM_ID: &str = "tray-quit-tolaria";
const QUIT_MENU_ITEM_LABEL: &str = "Quit Tolaria";
const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/128x128@2x.png");

/// Resident-mode flags shared between the run-event loop, the tray handlers and
/// the `set_tray_resident_mode` command.
#[derive(Debug, Default)]
pub(crate) struct TrayResidentState {
    enabled: AtomicBool,
    quitting: AtomicBool,
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
    sync_tray_icon(app_handle, enabled).map_err(|error| format!("Tray update failed: {error}"))?;
    restore_stranded_window(app_handle, enabled);
    Ok(())
}

pub(crate) fn handle_run_event(app_handle: &AppHandle, event: &RunEvent) {
    match event {
        RunEvent::ExitRequested { .. } => mark_quitting(app_handle),
        RunEvent::WindowEvent {
            label,
            event: WindowEvent::CloseRequested { api, .. },
            ..
        } => intercept_main_window_close(app_handle, label, api),
        _ => {}
    }
}

/// Bring the main window back and restore the regular macOS activation policy.
/// Shared by the tray, single-instance relaunches and deep links.
pub(crate) fn show_main_window(app_handle: &AppHandle) {
    set_regular_activation_policy(app_handle);

    let Some(window) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

fn intercept_main_window_close(app_handle: &AppHandle, label: &str, api: &CloseRequestApi) {
    if !should_intercept_close(label, is_enabled(app_handle), is_quitting(app_handle)) {
        return;
    }

    api.prevent_close();
    hide_main_window_to_tray(app_handle);
}

fn hide_main_window_to_tray(app_handle: &AppHandle) {
    if let Some(window) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.hide();
    }
    apply_accessory_policy_when_nothing_is_visible(app_handle);
}

fn apply_accessory_policy_when_nothing_is_visible(app_handle: &AppHandle) {
    let visible_labels = visible_window_labels(app_handle);
    let visible_labels: Vec<&str> = visible_labels.iter().map(String::as_str).collect();
    if should_use_accessory_policy(is_enabled(app_handle), &visible_labels) {
        set_accessory_activation_policy(app_handle);
    }
}

fn visible_window_labels(app_handle: &AppHandle) -> Vec<String> {
    app_handle
        .webview_windows()
        .into_iter()
        .filter(|(_, window)| window.is_visible().unwrap_or(false))
        .map(|(label, _)| label)
        .collect()
}

fn restore_stranded_window(app_handle: &AppHandle, enabled: bool) {
    if should_restore_hidden_window(enabled, is_main_window_visible(app_handle)) {
        show_main_window(app_handle);
    }
}

fn is_main_window_visible(app_handle: &AppHandle) -> bool {
    app_handle
        .get_webview_window(MAIN_WINDOW_LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
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
    let show =
        MenuItemBuilder::with_id(SHOW_MENU_ITEM_ID, SHOW_MENU_ITEM_LABEL).build(app_handle)?;
    let quit =
        MenuItemBuilder::with_id(QUIT_MENU_ITEM_ID, QUIT_MENU_ITEM_LABEL).build(app_handle)?;
    MenuBuilder::new(app_handle)
        .item(&show)
        .separator()
        .item(&quit)
        .build()
}

fn handle_tray_menu_event(app_handle: &AppHandle, event: MenuEvent) {
    match event.id().0.as_str() {
        SHOW_MENU_ITEM_ID => show_main_window(app_handle),
        QUIT_MENU_ITEM_ID => quit(app_handle),
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

fn quit(app_handle: &AppHandle) {
    mark_quitting(app_handle);
    app_handle.exit(0);
}

fn resident_state(app_handle: &AppHandle) -> tauri::State<'_, TrayResidentState> {
    app_handle.state()
}

fn is_enabled(app_handle: &AppHandle) -> bool {
    resident_state(app_handle).enabled.load(Ordering::SeqCst)
}

fn set_enabled(app_handle: &AppHandle, enabled: bool) {
    resident_state(app_handle)
        .enabled
        .store(enabled, Ordering::SeqCst);
}

fn is_quitting(app_handle: &AppHandle) -> bool {
    resident_state(app_handle).quitting.load(Ordering::SeqCst)
}

fn mark_quitting(app_handle: &AppHandle) {
    resident_state(app_handle)
        .quitting
        .store(true, Ordering::SeqCst);
}

#[cfg(target_os = "macos")]
fn set_activation_policy(app_handle: &AppHandle, policy: tauri::ActivationPolicy) {
    if let Err(error) = app_handle.set_activation_policy(policy) {
        log::warn!("Failed to update the macOS activation policy: {error}");
    }
}

#[cfg(target_os = "macos")]
fn set_regular_activation_policy(app_handle: &AppHandle) {
    set_activation_policy(app_handle, tauri::ActivationPolicy::Regular);
}

#[cfg(target_os = "macos")]
fn set_accessory_activation_policy(app_handle: &AppHandle) {
    set_activation_policy(app_handle, tauri::ActivationPolicy::Accessory);
}

#[cfg(not(target_os = "macos"))]
fn set_regular_activation_policy(_app_handle: &AppHandle) {}

#[cfg(not(target_os = "macos"))]
fn set_accessory_activation_policy(_app_handle: &AppHandle) {}

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
