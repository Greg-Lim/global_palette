# Tauri Hotkey Migration Plan

> Documentation-only migration phase. Do not start implementation until this
> plan is reviewed and accepted.

## Migration Status

- Planning branch: `migration/tauri-hotkey`.
- Current status: plan document first, implementation not started.
- Target integration point: before Phase 8 cutover in
  `docs/migration/tauri-migration-plan.md`.
- Last updated: 2026-05-26.

## Problem

The current Tauri path still routes activation and guide-mode hotkeys through
the custom Windows `RegisterHotKey` listener. That listener works while Omni
Palette is hidden or unfocused, but it is the wrong owner for guide-mode
shortcuts after the UI moves into Tauri WebViews.

When a WebView is focused, WebView/browser defaults can intercept shortcuts
such as print, reload, find, source, downloads, zoom, and developer tools. If
the app solves this by broadly disabling or replacing shortcut handling, it can
also disable useful app behavior. The migration needs clearer ownership:

- Background activation belongs to Rust/Tauri global shortcut handling.
- Focused guide-mode shortcuts belong to the guide WebView.
- Browser/WebView defaults should be disabled only where they conflict with
  Omni Palette behavior.

## Decision

Use one global shortcut system for Tauri background activation and local
focused keyboard handling for guide mode.

Proposed packages:

- `tauri-plugin-global-shortcut` for Rust-side activation registration and
  updates while Omni Palette is hidden or unfocused.
- `tauri-plugin-prevent-default` for disabling WebView browser default
  shortcuts that collide with Omni Palette behavior.

Do not introduce React. The Tauri frontend is Svelte, and this migration should
preserve the Svelte/Vite/Bun direction.

Do not use two competing global shortcut systems. In particular, do not keep
guide-mode activation, Escape, or captured command shortcuts registered through
the current custom Windows listener once the Tauri hotkey path is migrated.

Do not add a frontend hotkey package for guide mode. The guide surface only
needs to match the configured activation shortcut, Escape, and the captured
single shortcut. Implement this as a small local TypeScript helper that compares
focused `KeyboardEvent` values to Omni Palette's structured shortcut DTOs by
physical `event.code`.

Treat this migration as a behavior-preserving bugfix and architecture cleanup.
Do not change tray behavior, settings layout, palette visuals, command labels,
or unrelated user workflows.

## Sources

- Tauri global shortcut plugin:
  https://v2.tauri.app/plugin/global-shortcut/
- Tauri global shortcut JavaScript reference:
  https://tauri.app/reference/javascript/global-shortcut/
- WebView2 browser accelerator keys:
  https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/winrt/microsoft_web_webview2_core/corewebview2settings
- `tauri-plugin-prevent-default` docs:
  https://docs.rs/crate/tauri-plugin-prevent-default/latest

## Target Behavior

### Background Activation

- Pressing the configured activation shortcut while Omni Palette is hidden or
  unfocused opens the palette.
- Pressing the configured activation shortcut while the palette is visible
  hides it, preserving current palette toggle behavior.
- Ignored foreground apps still receive the activation shortcut as passthrough
  instead of opening Omni Palette.
- Runtime settings can still record, save, and reset the activation shortcut.
  Saving a new activation shortcut updates the active Tauri global shortcut
  registration and rolls back if persistence fails.

### Guide Mode

- Guide mode supports single shortcut commands only.
- Starting guide mode for a single shortcut command hides the palette, focuses
  the target window, and shows the guide WebView.
- While the guide WebView is focused:
  - The configured activation shortcut completes the active guide command.
  - Escape cancels guide without forwarding a shortcut.
  - The shown captured shortcut cancels guide and forwards that shortcut to the
    target app.
- Any unrelated key or shortcut has no Omni Palette behavior.
- Shortcut-sequence commands are not guideable for now. If command behavior is
  set to Guide, sequence commands still execute normally when selected from the
  palette.
- Palette rows may still show shortcut-sequence text, but they should not expose
  guide capture behavior.
- Guide-mode shortcuts are not registered as global hotkeys.

### WebView Defaults

- Browser defaults that conflict with Omni Palette guide handling, such as
  print, find, reload, source, downloads, and zoom, should not steal focused
  guide-mode shortcuts in the guide window.
- Keep unrelated WebView behavior unchanged unless a direct conflict is
  documented.
- Keep debug-friendly reload and developer tools behavior in debug builds when
  practical.
- If plugin limitations require a broader WebView default-prevention setting
  than guide-window-only, use the narrowest safe setting and document why it had
  to be broader.

### Error Handling

- Activation shortcut registration failure on startup should not abort app
  startup. Launch the app with hotkey status showing a controlled error.
- If Settings save cannot register a new activation shortcut, keep the previous
  active shortcut and leave the unsaved draft visible so the user can change it.
- If Settings save registers the shortcut but cannot persist `config.toml`, fail
  the save, roll back to the previous active shortcut, and leave the unsaved
  draft visible.
- If guide captured-shortcut forwarding fails, make the failure noisy through
  logs, toast, or status surfaces. The debug panel should remain value-oriented
  instead of becoming the primary user-facing error surface.
- Settings save failures should show a toast or equivalent visible status using
  the existing settings feedback pattern.

## Implementation Outline

Implement Phase 7A as one PR-sized migration. Keep commits scoped by subsystem
where convenient, but do not split the migration into separate release phases.

### Dependency And Ownership Setup

- Add `tauri-plugin-global-shortcut` to the Tauri Rust crate.
- Add `tauri-plugin-prevent-default` with Windows platform support.
- Register both plugins from the Tauri builder before app setup.
- Keep the existing custom Windows hotkey listener for egui until Phase 8
  removes egui.

### Tauri Activation Shortcut Bridge

- Replace Tauri's use of `HotkeyBridge::start` with a Tauri global shortcut
  bridge that owns only the activation shortcut.
- Preserve the existing activation status DTO shape where possible, including
  running state, activation hint, counts, last event, and last error.
- Keep foreground context lookup before opening the palette so ignored-app
  passthrough and command filtering continue to use the correct target window.
- Convert between Omni Palette `KeyboardShortcut` values and
  `tauri-plugin-global-shortcut` shortcut values in one backend helper.

### Focused Guide Shortcut Handling

- Add a general structured shortcut DTO for frontend matching. It should carry
  modifier booleans, the runtime key, and display text. Keep settings-specific
  activation shortcut naming as an alias or wrapper if needed.
- Extend guide status data with the structured configured activation shortcut
  and the captured single shortcut.
- Add focused guide invokes for:
  - completing the active guide command;
  - cancelling guide;
  - cancelling guide and forwarding the captured shortcut.
- Update `Guide.svelte` to match focused `keydown` events against the configured
  activation shortcut, Escape, and the captured shortcut.
- Prevent default browser behavior only for guide shortcuts that Omni Palette
  handles.
- For recognized guide shortcuts, call both `preventDefault()` and
  `stopPropagation()`.
- Treat shortcut-sequence commands as not guideable; they should execute
  normally under Guide command behavior.

### WebView Default Prevention

- Configure `tauri-plugin-prevent-default` narrowly around browser shortcut
  collisions in the guide window first.
- Avoid disabling text-editing shortcuts that settings forms need, such as copy,
  paste, undo, select-all, text navigation, and ordinary Tab movement.
- Document any intentional debug-build exceptions.
- Preserve DevTools and reload shortcuts in development builds where practical.

### Cutover Cleanup Preparation

- Mark the old Tauri guide global-hotkey path as removed from the Tauri runtime.
- Keep the old Windows listener files in place for egui until Phase 8.
- Update `docs/migration/tauri-parity-checklist.md` with manual checks for the
  new hotkey ownership behavior.

## Testing Plan

Rust tests:

- Tauri activation bridge registers the configured activation shortcut on
  startup.
- Activation shortcut registration failure on startup records controlled hotkey
  status without aborting app startup.
- Released shortcut events do not trigger palette activation.
- Saving a new activation shortcut updates the active registration.
- Failed shortcut registration during Settings save leaves the previous active
  shortcut registered and keeps the unsaved draft visible.
- If config persistence fails after a shortcut update, the previous shortcut is
  restored and the unsaved draft remains visible.
- Ignored foreground apps still receive passthrough behavior.
- Guide completion and cancellation no longer depend on guide global hotkey
  registration.
- Shortcut-sequence commands are not guideable and execute normally when command
  behavior is Guide.

TypeScript and Svelte tests:

- Browser `KeyboardEvent` values map to the same structured shortcut DTO used by
  runtime settings.
- Guide key handling matches activation and captured shortcuts exactly.
- Guide key handling ignores unrelated keys.
- Matched guide shortcuts call `preventDefault()` and `stopPropagation()`.
- Shortcut matching uses physical `event.code`, not typed character
  `event.key`.

Manual checks:

- With no Omni Palette window visible, pressing the activation shortcut opens
  the palette.
- With the palette visible, pressing the activation shortcut hides it.
- With guide visible, pressing the activation shortcut runs the stored command.
- With guide visible, pressing Escape cancels guide.
- With guide visible, pressing the shown captured shortcut cancels guide and
  forwards the shortcut to the target app.
- With command behavior set to Guide, selecting a shortcut-sequence command from
  the palette executes the command normally instead of opening guide.
- WebView defaults such as print, find, reload, source, downloads, and zoom do
  not steal Omni Palette guide shortcuts.

## Non-Goals

- Do not remove egui during this phase.
- Do not remove the old Windows hotkey listener while egui still depends on it.
- Do not reintroduce React or React hotkey packages.
- Do not add a frontend hotkey package for guide-mode matching.
- Do not use the JavaScript API of `@tauri-apps/plugin-global-shortcut` for
  guide-mode shortcuts.
- Do not support shortcut-sequence commands in guide mode during this phase.
- Do not redesign command execution, extension loading, or guide UI visuals.
- Do not change tray behavior, settings layout, palette visuals, command labels,
  or unrelated user workflows.

## Acceptance Criteria

- The migration doc is reviewed before implementation starts.
- Tauri uses one global shortcut owner for activation.
- Guide-mode shortcut handling is focused and WebView-owned.
- Shortcut-sequence commands execute normally and do not enter guide mode.
- Browser defaults no longer steal focused guide-mode shortcuts.
- egui remains functional until Phase 8 cutover.
- All Rust and frontend checks pass after implementation.
