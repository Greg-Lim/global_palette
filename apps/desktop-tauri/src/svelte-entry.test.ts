import { describe, expect, it } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const srcDir = import.meta.dir;
const appRoot = join(srcDir, "..");

describe("Svelte frontend entrypoint", () => {
  it("loads the Svelte TypeScript entrypoint from index.html", () => {
    const indexHtml = readFileSync(join(appRoot, "index.html"), "utf8");

    expect(indexHtml).toContain('src="/src/main.ts"');
    expect(indexHtml).not.toContain("/src/main.tsx");
    expect(existsSync(join(srcDir, "main.ts"))).toBe(true);
    expect(existsSync(join(srcDir, "main.tsx"))).toBe(false);
  });

  it("mounts the palette or guide Svelte app from the entrypoint", () => {
    const mainPath = join(srcDir, "main.ts");

    expect(existsSync(mainPath)).toBe(true);

    if (!existsSync(mainPath)) {
      return;
    }

    const mainSource = readFileSync(mainPath, "utf8");
    expect(mainSource).toContain('from "svelte"');
    expect(mainSource).toContain('from "./App.svelte"');
    expect(mainSource).toContain('from "./Guide.svelte"');
    expect(mainSource).toContain('from "./Settings.svelte"');
    expect(mainSource).toContain('from "./DebugOverlay.svelte"');
    expect(mainSource).toContain("getCurrentWindow().label");
    expect(mainSource).toContain('label === "settings"');
    expect(mainSource).toContain('label === "debug"');
    expect(mainSource).toContain("mount(Component,");
  });

  it("keeps settings controls out of the hotkey palette surface", () => {
    const appSource = readFileSync(join(srcDir, "App.svelte"), "utf8");

    expect(appSource).toContain("openSettingsFromPalette");
    expect(appSource).toContain("refreshExtensionsFromPalette");
    expect(appSource).not.toContain("Backend:");
    expect(appSource).not.toContain("activeView");
    expect(appSource).not.toContain("Activation shortcut");
    expect(appSource).not.toContain("Save settings");
  });

  it("keeps the palette to one hidden results scroller with fixed header and edge fades", () => {
    const appSource = readFileSync(join(srcDir, "App.svelte"), "utf8");

    expect(appSource).toContain("configureRuntimeAppearanceTheme");
    expect(appSource).toContain("palette-shell");
    expect(appSource).toContain("palette-surface");
    expect(appSource).toContain("palette-row-selected");
    expect(appSource).toContain("h-screen overflow-hidden");
    expect(appSource).toContain('id="command-search"');
    expect(appSource).toContain('autocomplete="off"');
    expect(appSource).toContain('autocapitalize="none"');
    expect(appSource).toContain('autocorrect="off"');
    expect(appSource).toContain('spellcheck={false}');
    expect(appSource).toContain('aria-autocomplete="none"');
    expect(appSource).toContain("palette-results-scroll");
    expect(appSource).toContain("bind:this={resultsScroller}");
    expect(appSource).toContain("handleResultsScroll");
    expect(appSource).toContain("resetResultsScrollToTop");
    expect(appSource).toContain("scrollTo({ top: nextScrollTop, behavior: \"smooth\" })");
    expect(appSource).toContain("scrollbar-width: none");
    expect(appSource).toContain(".palette-results-scroll::-webkit-scrollbar");
    expect(appSource).toContain("showTopFade");
    expect(appSource).toContain("showBottomFade");
    expect(appSource).toContain("palette-top-fade");
    expect(appSource).toContain("pb-14");
    expect(appSource).toContain("palette-bottom-fade");
    expect(appSource).not.toMatch(/(?:bg|text|border)-zinc/);
    expect(appSource).not.toContain("Run selected");
    expect(appSource).not.toContain("Math.max(rows.length - 1, 0)} commands");
    expect(appSource).not.toContain("sticky top-0");
    expect(appSource).not.toContain("max-h-[420px] overflow-y-auto");
  });

  it("renders Phase 6C settings navigation and surfaces in the settings window", () => {
    const settingsPath = join(srcDir, "Settings.svelte");

    expect(existsSync(settingsPath)).toBe(true);

    if (!existsSync(settingsPath)) {
      return;
    }

    const settingsSource = readFileSync(settingsPath, "utf8");
    expect(settingsSource).toContain("General");
    expect(settingsSource).toContain("Manage Extensions");
    expect(settingsSource).toContain("Marketplace");
    expect(settingsSource).toContain("Activation shortcut");
    expect(settingsSource).toContain("Record");
    expect(settingsSource).toContain("Reset");
    expect(settingsSource).toContain("Command behavior");
    expect(settingsSource).toContain("Pop up debugger");
    expect(settingsSource).toContain("showDebugOverlay");
    expect(settingsSource).toContain("settings-section");
    expect(settingsSource).toContain("settings-row");
    expect(settingsSource).toContain("settings-row-label");
    expect(settingsSource).toContain("settings-row-control");
    expect(settingsSource).toContain("applySettingsAppearanceTheme");
    expect(settingsSource).toContain("configureAppearanceTheme");
    expect(settingsSource).toContain("settings-card");
    expect(settingsSource).toContain("settings-modal-panel");
    expect(settingsSource).toContain('from "./ToggleSwitch.svelte"');
    expect(settingsSource).not.toMatch(/(?:bg|text|border)-(?:zinc|red|emerald|amber)-/);
    expect(settingsSource).not.toContain(
      '<legend class="px-1 text-sm font-medium text-zinc-200">Appearance</legend>',
    );
    expect(settingsSource).not.toContain("Activation shortcut\n              </legend>");
    expect(settingsSource).not.toContain("Command behavior</legend>");
    expect(settingsSource).toContain("Bundled Defaults");
    expect(settingsSource).toContain("Downloaded Extensions");
    expect(settingsSource).toContain("No downloaded extensions installed yet.");
    expect(settingsSource).toContain("extensionStatusLabel");
    expect(settingsSource).toContain("extension-status-pill");
    expect(settingsSource).toContain("ToggleSwitch");
    expect(settingsSource).toContain("ariaLabel={`Toggle ${extension.name}`}");
    expect(settingsSource).not.toContain("extensionToggleTrackClass");
    expect(settingsSource).not.toContain("extensionToggleThumbClass");
    expect(settingsSource).not.toContain("settings-checkbox");
    expect(settingsSource).toContain("Catalog source");
    expect(settingsSource).toContain("Save Source");
    expect(settingsSource).toContain("Refresh Catalog");
    expect(settingsSource).toContain("Available Extensions");
    expect(settingsSource).toContain("Search catalog");
    expect(settingsSource).toContain("Install");
    expect(settingsSource).toContain("Save settings");
    expect(settingsSource).toContain("getExtensionSettings");
    expect(settingsSource).toContain("saveExtensionSettings");
    expect(settingsSource).toContain("extensionSettingsPanel");
    expect(settingsSource).toContain("Reset Defaults");
    expect(settingsSource).toContain("Add Entry");
    expect(settingsSource).not.toContain("Extension settings panels arrive in Phase 6C.3.");
  });

  it("provides a Handy-style toggle switch component for settings booleans", () => {
    const togglePath = join(srcDir, "ToggleSwitch.svelte");

    expect(existsSync(togglePath)).toBe(true);

    if (!existsSync(togglePath)) {
      return;
    }

    const toggleSource = readFileSync(togglePath, "utf8");
    expect(toggleSource).toContain("export let checked");
    expect(toggleSource).toContain("export let ariaLabel");
    expect(toggleSource).toContain("export let onToggle");
    expect(toggleSource).toContain('type="checkbox"');
    expect(toggleSource).toContain('role="switch"');
    expect(toggleSource).toContain('class="sr-only"');
    expect(toggleSource).toContain("toggle-switch-track");
    expect(toggleSource).toContain("toggle-switch-thumb");
    expect(toggleSource).toContain("toggle-switch-spinner");
    expect(toggleSource).toContain("onchange={handleChange}");
  });

  it("renders a separate debug overlay surface", () => {
    const debugPath = join(srcDir, "DebugOverlay.svelte");

    expect(existsSync(debugPath)).toBe(true);

    if (!existsSync(debugPath)) {
      return;
    }

    const debugSource = readFileSync(debugPath, "utf8");
    expect(debugSource).toContain("getDebugSnapshot");
    expect(debugSource).toContain("const DEBUG_SNAPSHOT_REFRESH_MS = 1000");
    expect(debugSource).toContain(
      "window.setInterval(refreshDebugSnapshot, DEBUG_SNAPSHOT_REFRESH_MS)",
    );
    expect(debugSource).toContain("let refreshInFlight = false");
    expect(debugSource).toContain("let paletteRowsOpen = true");
    expect(debugSource).toContain("let backgroundWindowsOpen = true");
    expect(debugSource).toContain("let showScoreBreakdown = false");
    expect(debugSource).toContain("Foreground");
    expect(debugSource).toContain("Interaction");
    expect(debugSource).toContain("Command Candidates");
    expect(debugSource).toContain("Palette Filter");
    expect(debugSource).toContain("aria-expanded={paletteRowsOpen}");
    expect(debugSource).toContain("Score breakdown");
    expect(debugSource).toContain("score_breakdown");
    expect(debugSource).toContain("% focus");
    expect(debugSource).toContain("% priority");
    expect(debugSource).toContain("% favorite");
    expect(debugSource).toContain("Background Windows");
    expect(debugSource).toContain("aria-expanded={backgroundWindowsOpen}");
    expect(debugSource).not.toContain("closeDebugOverlay");
    expect(debugSource).not.toContain("Refreshing...");
    expect(debugSource).not.toContain("onclick={refreshDebugSnapshot}");
  });

  it("renders guide mode without an outer shadow and with larger keycaps", () => {
    const guidePath = join(srcDir, "Guide.svelte");

    expect(existsSync(guidePath)).toBe(true);

    if (!existsSync(guidePath)) {
      return;
    }

    const guideSource = readFileSync(guidePath, "utf8");
    expect(guideSource).toContain("configureRuntimeAppearanceTheme");
    expect(guideSource).toContain("flex h-screen overflow-hidden");
    expect(guideSource).toContain("guide-panel");
    expect(guideSource).not.toContain("shadow-2xl");
    expect(guideSource).toContain("min-h-24 min-w-32");
    expect(guideSource).toContain("px-8 py-6");
    expect(guideSource).toContain("text-2xl");
    expect(guideSource).not.toMatch(/(?:bg|text|border)-zinc/);
  });

  it("declares separate palette, settings, and guide Tauri windows", () => {
    const config = JSON.parse(
      readFileSync(join(appRoot, "src-tauri", "tauri.conf.json"), "utf8"),
    );
    const windows = config.app.windows as Array<Record<string, unknown>>;
    const mainWindow = windows.find((window) => window.label === "main");
    const settingsWindow = windows.find((window) => window.label === "settings");
    const guideWindow = windows.find((window) => window.label === "guide");
    const debugWindow = windows.find((window) => window.label === "debug");

    expect(mainWindow).toMatchObject({
      label: "main",
      width: 780,
      height: 600,
      decorations: false,
      visible: false,
    });
    expect(settingsWindow).toMatchObject({
      label: "settings",
      title: "Omni Palette Settings",
      decorations: true,
      resizable: true,
      visible: false,
    });
    expect(guideWindow).toMatchObject({
      label: "guide",
      height: 320,
      visible: false,
    });
    expect(debugWindow).toMatchObject({
      label: "debug",
      title: "Omni Palette Debug",
      decorations: true,
      resizable: true,
      visible: false,
    });
  });

  it("grants default Tauri permissions to every local app window", () => {
    const config = JSON.parse(
      readFileSync(join(appRoot, "src-tauri", "tauri.conf.json"), "utf8"),
    );
    const capability = JSON.parse(
      readFileSync(join(appRoot, "src-tauri", "capabilities", "default.json"), "utf8"),
    );
    const windowLabels = (config.app.windows as Array<Record<string, unknown>>).map(
      (window) => window.label,
    );

    expect(capability.permissions).toContain("core:default");
    expect(capability.windows).toEqual(windowLabels);
  });
});
