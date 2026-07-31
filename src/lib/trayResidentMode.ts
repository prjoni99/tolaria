import { invoke } from '@tauri-apps/api/core'
import { isTauri } from '../mock-tauri'
import { trackEvent } from './telemetry'
import type { Settings } from '../types'

export const DEFAULT_TRAY_RESIDENT_MODE = false
const SET_TRAY_RESIDENT_MODE_COMMAND = 'set_tray_resident_mode'

/** Resolve the effective tray resident mode preference, defaulting to off. */
export function isTrayResidentModeEnabled(settings: Settings): boolean {
  return settings.tray_resident_mode_enabled ?? DEFAULT_TRAY_RESIDENT_MODE
}

/** Ask the Rust side to create or destroy the tray icon without an app restart. */
export async function applyTrayResidentMode(enabled: boolean): Promise<void> {
  if (!isTauri()) return

  try {
    await invoke(SET_TRAY_RESIDENT_MODE_COMMAND, { enabled })
  } catch (err) {
    console.warn('[tray] Failed to apply tray resident mode:', err)
  }
}

export function trackTrayResidentModeToggled(enabled: boolean): void {
  trackEvent('tray_resident_mode_toggled', { enabled: enabled ? 1 : 0 })
}

/** Apply and instrument a tray resident mode change; no-op when the value is unchanged. */
export async function syncTrayResidentMode(previous: Settings, next: Settings): Promise<void> {
  const nextEnabled = isTrayResidentModeEnabled(next)
  if (isTrayResidentModeEnabled(previous) === nextEnabled) return

  trackTrayResidentModeToggled(nextEnabled)
  await applyTrayResidentMode(nextEnabled)
}
