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

- Starting guide mode hides the palette, focuses the target window, and shows
  the guide WebView.
- While the guide WebView is focused:
  - The activation shortcut completes the active guide command.
  - Escape cancels guide without forwarding a shortcut.
  - The shown captured shortcut cancels guide and forwards that shortcut to the
    target app.
- Shortcut-sequence guide commands remain non-capturing. They show the sequence
  text and use the activation shortcut to run the stored command.
- Guide-mode shortcuts are not registered as global hotkeys.

### WebView Defaults

- Browser defaults that conflict with Omni Palette, such as print, find,
  reload, source, downloads, and zoom, should not steal focused guide-mode
  shortcuts.
- Keep unrelated WebView behavior unchanged unless a direct conflict is
  documented.
- Keep debug-friendly reload and developer tools behavior in debug builds when
  practical.

## Implementation Outline

### Phase 7A.1: Dependency And Ownership Setup

- Add `tauri-plugin-global-shortcut` to the Tauri Rust crate.
- Add `tauri-plugin-prevent-default` with Windows platform support.
- Register both plugins from the Tauri builder before app setup.
- Keep the existing custom Windows hotkey listener for egui until Phase 8
  removes egui.

### Phase 7A.2: Tauri Activation Shortcut Bridge

- Replace Tauri's use of `HotkeyBridge::start` with a Tauri global shortcut
  bridge that owns only the activation shortcut.
- Preserve the existing activation status DTO shape where possible, including
  running state, activation hint, counts, last event, and last error.
- Keep foreground context lookup before opening the palette so ignored-app
  passthrough and command filtering continue to use the correct target window.
- Convert between Omni Palette `KeyboardShortcut` values and
  `tauri-plugin-global-shortcut` shortcut values in one backend helper.

### Phase 7A.3: Focused Guide Shortcut Handling

- Extend guide status data with the structured activation shortcut and optional
  captured shortcut.
- Add focused guide invokes for:
  - completing the active guide command;
  - cancelling guide;
  - cancelling guide and forwarding the captured shortcut.
- Update `Guide.svelte` to match focused `keydown` events against the structured
  activation and captured shortcut data.
- Prevent default browser behavior for guide shortcuts that Omni Palette handles.

### Phase 7A.4: WebView Default Prevention

- Configure `tauri-plugin-prevent-default` narrowly around browser shortcut
  collisions.
- Avoid disabling text-editing shortcuts that settings forms need, such as copy,
  paste, undo, select-all, text navigation, and ordinary Tab movement.
- Document any intentional debug-build exceptions.

### Phase 7A.5: Cutover Cleanup Preparation

- Mark the old Tauri guide global-hotkey path as removed from the Tauri runtime.
- Keep the old Windows listener files in place for egui until Phase 8.
- Update `docs/migration/tauri-parity-checklist.md` with manual checks for the
  new hotkey ownership behavior.

## Testing Plan

Rust tests:

- Tauri activation bridge registers the configured activation shortcut on
  startup.
- Released shortcut events do not trigger palette activation.
- Saving a new activation shortcut updates the active registration.
- If config persistence fails after a shortcut update, the previous shortcut is
  restored.
- Ignored foreground apps still receive passthrough behavior.
- Guide completion and cancellation no longer depend on guide global hotkey
  registration.

TypeScript and Svelte tests:

- Browser `KeyboardEvent` values map to the same structured shortcut DTO used by
  runtime settings.
- Guide key handling matches activation and captured shortcuts exactly.
- Guide key handling ignores unrelated keys.
- Matched guide shortcuts call `preventDefault()`.

Manual checks:

- With no Omni Palette window visible, pressing the activation shortcut opens
  the palette.
- With the palette visible, pressing the activation shortcut hides it.
- With guide visible, pressing the activation shortcut runs the stored command.
- With guide visible, pressing Escape cancels guide.
- With guide visible, pressing the shown captured shortcut cancels guide and
  forwards the shortcut to the target app.
- WebView defaults such as print, find, reload, source, downloads, and zoom do
  not steal Omni Palette guide shortcuts.

## Non-Goals

- Do not remove egui during this phase.
- Do not remove the old Windows hotkey listener while egui still depends on it.
- Do not reintroduce React or React hotkey packages.
- Do not use the JavaScript API of `@tauri-apps/plugin-global-shortcut` for
  guide-mode shortcuts.
- Do not redesign command execution, extension loading, or guide UI visuals.

## Acceptance Criteria

- The migration doc is reviewed before implementation starts.
- Tauri uses one global shortcut owner for activation.
- Guide-mode shortcut handling is focused and WebView-owned.
- Browser defaults no longer steal focused guide-mode shortcuts.
- egui remains functional until Phase 8 cutover.
- All Rust and frontend checks pass after implementation.
