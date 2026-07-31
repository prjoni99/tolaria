//! Pure decision rules for tray resident mode.
//!
//! No Tauri types live here so every rule stays unit-testable, which keeps
//! `tray.rs` a thin layer of platform glue.

use crate::window_state::MAIN_WINDOW_LABEL;

/// What has to happen to the tray icon to match the current preference.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum TrayAction {
    Create,
    Destroy,
    None,
}

/// Only the main window turns a close into a hide. Note and AI workspace
/// windows keep their normal close behavior, and a real quit passes through.
pub(crate) fn should_intercept_close(
    window_label: &str,
    resident_enabled: bool,
    quitting: bool,
) -> bool {
    resident_enabled && !quitting && window_label == MAIN_WINDOW_LABEL
}

/// macOS may leave the Dock and Cmd+Tab only once nothing is left on screen.
/// Going accessory with a note window still open would orphan that window.
pub(crate) fn should_use_accessory_policy(
    resident_enabled: bool,
    visible_window_labels: &[&str],
) -> bool {
    resident_enabled && visible_window_labels.is_empty()
}

pub(crate) fn tray_should_exist(resident_enabled: bool, tray_present: bool) -> TrayAction {
    match (resident_enabled, tray_present) {
        (true, false) => TrayAction::Create,
        (false, true) => TrayAction::Destroy,
        _ => TrayAction::None,
    }
}

/// Turning the preference off while the window is hidden must not strand the
/// user with neither a tray icon nor a window.
pub(crate) fn should_restore_hidden_window(
    resident_enabled: bool,
    main_window_visible: bool,
) -> bool {
    !resident_enabled && !main_window_visible
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTE_WINDOW_LABEL: &str = "note-inbox";
    const AI_WORKSPACE_WINDOW_LABEL: &str = "ai-workspace";

    #[test]
    fn intercepts_only_the_main_window_close_while_resident_and_not_quitting() {
        let cases = [
            (MAIN_WINDOW_LABEL, true, false, true),
            (MAIN_WINDOW_LABEL, true, true, false),
            (MAIN_WINDOW_LABEL, false, false, false),
            (MAIN_WINDOW_LABEL, false, true, false),
            (NOTE_WINDOW_LABEL, true, false, false),
            (NOTE_WINDOW_LABEL, true, true, false),
            (NOTE_WINDOW_LABEL, false, false, false),
            (NOTE_WINDOW_LABEL, false, true, false),
            (AI_WORKSPACE_WINDOW_LABEL, true, false, false),
            (AI_WORKSPACE_WINDOW_LABEL, false, false, false),
        ];

        for (label, resident_enabled, quitting, expected) in cases {
            assert_eq!(
                should_intercept_close(label, resident_enabled, quitting),
                expected,
                "label={label} resident={resident_enabled} quitting={quitting}"
            );
        }
    }

    #[test]
    fn uses_accessory_policy_only_when_resident_and_nothing_is_visible() {
        let cases: [(bool, &[&str], bool); 6] = [
            (true, &[], true),
            (true, &[MAIN_WINDOW_LABEL], false),
            (true, &[NOTE_WINDOW_LABEL], false),
            (true, &[MAIN_WINDOW_LABEL, NOTE_WINDOW_LABEL], false),
            (false, &[], false),
            (false, &[NOTE_WINDOW_LABEL], false),
        ];

        for (resident_enabled, visible, expected) in cases {
            assert_eq!(
                should_use_accessory_policy(resident_enabled, visible),
                expected,
                "resident={resident_enabled} visible={visible:?}"
            );
        }
    }

    #[test]
    fn creates_or_destroys_the_tray_to_match_the_preference() {
        assert_eq!(tray_should_exist(true, false), TrayAction::Create);
        assert_eq!(tray_should_exist(false, true), TrayAction::Destroy);
        assert_eq!(tray_should_exist(true, true), TrayAction::None);
        assert_eq!(tray_should_exist(false, false), TrayAction::None);
    }

    #[test]
    fn restores_a_hidden_window_only_when_resident_mode_is_turned_off() {
        assert!(should_restore_hidden_window(false, false));
        assert!(!should_restore_hidden_window(false, true));
        assert!(!should_restore_hidden_window(true, false));
        assert!(!should_restore_hidden_window(true, true));
    }
}
