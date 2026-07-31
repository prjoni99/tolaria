//! Pure decision rules for tray resident mode.
//!
//! No Tauri types live here so every rule stays unit-testable, which keeps
//! `tray.rs` a thin layer of platform glue.

/// What has to happen to the tray icon to match the current preference.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum TrayAction {
    Create,
    Destroy,
    None,
}

pub(crate) fn tray_should_exist(resident_enabled: bool, tray_present: bool) -> TrayAction {
    match (resident_enabled, tray_present) {
        (true, false) => TrayAction::Create,
        (false, true) => TrayAction::Destroy,
        _ => TrayAction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_or_destroys_the_tray_to_match_the_preference() {
        assert_eq!(tray_should_exist(true, false), TrayAction::Create);
        assert_eq!(tray_should_exist(false, true), TrayAction::Destroy);
        assert_eq!(tray_should_exist(true, true), TrayAction::None);
        assert_eq!(tray_should_exist(false, false), TrayAction::None);
    }
}
