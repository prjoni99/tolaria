import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  applyTrayResidentMode,
  isTrayResidentModeEnabled,
  syncTrayResidentMode,
} from './trayResidentMode'
import type { Settings } from '../types'

const { invoke, isTauri, trackEvent } = vi.hoisted(() => ({
  invoke: vi.fn().mockResolvedValue(undefined),
  isTauri: vi.fn().mockReturnValue(true),
  trackEvent: vi.fn(),
}))

vi.mock('@tauri-apps/api/core', () => ({ invoke }))
vi.mock('../mock-tauri', () => ({ isTauri }))
vi.mock('./telemetry', () => ({ trackEvent }))

function settings(overrides: Partial<Settings> = {}): Settings {
  return {
    auto_pull_interval_minutes: null,
    telemetry_consent: null,
    crash_reporting_enabled: null,
    analytics_enabled: null,
    anonymous_id: null,
    release_channel: null,
    ...overrides,
  }
}

describe('trayResidentMode', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    isTauri.mockReturnValue(true)
    invoke.mockResolvedValue(undefined)
  })

  it('defaults to disabled when the preference is absent', () => {
    expect(isTrayResidentModeEnabled(settings())).toBe(false)
    expect(isTrayResidentModeEnabled(settings({ tray_resident_mode_enabled: null }))).toBe(false)
  })

  it('reads the stored preference when present', () => {
    expect(isTrayResidentModeEnabled(settings({ tray_resident_mode_enabled: true }))).toBe(true)
    expect(isTrayResidentModeEnabled(settings({ tray_resident_mode_enabled: false }))).toBe(false)
  })

  it('invokes the native command when applying the mode', async () => {
    await applyTrayResidentMode(true)

    expect(invoke).toHaveBeenCalledWith('set_tray_resident_mode', { enabled: true })
  })

  it('skips the native command outside Tauri', async () => {
    isTauri.mockReturnValue(false)

    await applyTrayResidentMode(true)

    expect(invoke).not.toHaveBeenCalled()
  })

  it('swallows native command failures', async () => {
    invoke.mockRejectedValue(new Error('no tray'))
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})

    await expect(applyTrayResidentMode(false)).resolves.toBeUndefined()

    expect(warn).toHaveBeenCalled()
    warn.mockRestore()
  })

  it('tracks and applies the change when the preference flips on', async () => {
    await syncTrayResidentMode(settings(), settings({ tray_resident_mode_enabled: true }))

    expect(trackEvent).toHaveBeenCalledWith('tray_resident_mode_toggled', { enabled: 1 })
    expect(invoke).toHaveBeenCalledWith('set_tray_resident_mode', { enabled: true })
  })

  it('tracks and applies the change when the preference flips off', async () => {
    await syncTrayResidentMode(
      settings({ tray_resident_mode_enabled: true }),
      settings({ tray_resident_mode_enabled: false }),
    )

    expect(trackEvent).toHaveBeenCalledWith('tray_resident_mode_toggled', { enabled: 0 })
    expect(invoke).toHaveBeenCalledWith('set_tray_resident_mode', { enabled: false })
  })

  it('does nothing when the preference is unchanged', async () => {
    await syncTrayResidentMode(
      settings({ tray_resident_mode_enabled: true }),
      settings({ tray_resident_mode_enabled: true }),
    )

    expect(trackEvent).not.toHaveBeenCalled()
    expect(invoke).not.toHaveBeenCalled()
  })
})
