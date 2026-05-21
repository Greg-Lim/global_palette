<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import ToggleSwitch from "./ToggleSwitch.svelte";

  import type {
    ActivationShortcut,
    CatalogEntry,
    ExtensionSettingItem,
    ExtensionRow,
    ExtensionSettingsSection,
    ExtensionSettingsSchema,
    ExtensionSettingsTarget,
    ExtensionSettingsValues,
    ExtensionsBootstrap,
    RuntimeSettings,
  } from "./commands";
  import {
    addExtensionSettingListEntry,
    activationShortcutFromKeyboardEvent,
    applyCatalogRefreshResult,
    applyExtensionMutationResult,
    applyExtensionSettingsSaveResult,
    applyRuntimeSettingsSaveResult,
    configureAppearanceTheme,
    copyExtensionSettingsValues,
    defaultExtensionSettingsValues,
    discardRuntimeSettingsDraft,
    extensionSettingsAreDirty,
    extensionSettingsSaveRequestFromDraft,
    extensionSettingsSections,
    filterCatalogEntries,
    formatActivationShortcut,
    paletteApi,
    removeExtensionSettingListEntry,
    runtimeSettingsAreDirty,
    runtimeSettingsSaveRequestFromDraft,
    updateExtensionSettingListEntry,
    updateExtensionSettingToggle,
  } from "./commands";

  type SettingsPage = "general" | "extensions" | "marketplace";
  type ExtensionSettingsPanel = {
    target: ExtensionSettingsTarget;
    schema: ExtensionSettingsSchema;
    saved: ExtensionSettingsValues;
    draft: ExtensionSettingsValues;
    saving: boolean;
    message: string | null;
    failed: boolean;
  };

  const EXTENSION_STATUS_RESERVED_LABEL = "Disabled";
  const CATALOG_ACTION_RESERVED_LABEL = "Installing...";
  const RECORD_SHORTCUT_RESERVED_LABEL = "Recording...";
  const HEADER_RELOAD_RESERVED_LABEL = "Reload extensions";
  const SAVE_SOURCE_RESERVED_LABEL = "Save Source";
  const REFRESH_CATALOG_RESERVED_LABEL = "Refresh Catalog";
  const CATALOG_RELOAD_RESERVED_LABEL = "Reload Extensions";
  const EXTENSION_SETTINGS_RESERVED_LABEL = "Loading...";
  const SAVE_SETTINGS_RESERVED_LABEL = "Save settings";
  const SAVE_EXTENSION_SETTINGS_RESERVED_LABEL = "Save Settings";

  let activeSettingsPage: SettingsPage = "general";
  let settingsSaved: RuntimeSettings | null = null;
  let settingsDraft: RuntimeSettings | null = null;
  let defaultActivationShortcut: ActivationShortcut | null = null;
  let extensionsBootstrap: ExtensionsBootstrap | null = null;
  let settingsConfigPath: string | null = null;
  let settingsConfigError: string | null = null;
  let settingsLoading = true;
  let extensionsLoading = true;
  let settingsSaving = false;
  let settingsReloading = false;
  let catalogRefreshing = false;
  let catalogInstallingId: string | null = null;
  let catalogEntries: CatalogEntry[] = [];
  let catalogQuery = "";
  let recordingActivationShortcut = false;
  let extensionMutationKey: string | null = null;
  let extensionSettingsLoadingKey: string | null = null;
  let extensionSettingsPanel: ExtensionSettingsPanel | null = null;
  let settingsMessage: string | null = null;
  let settingsFailed = false;
  let stopSystemAppearanceWatcher: (() => void) | null = null;

  $: settingsDirty = runtimeSettingsAreDirty(settingsSaved, settingsDraft);
  $: visibleCatalogEntries = filterCatalogEntries(catalogEntries, catalogQuery);
  $: extensionSettingsDirty = extensionSettingsPanel
    ? extensionSettingsAreDirty(extensionSettingsPanel.saved, extensionSettingsPanel.draft)
    : false;
  $: extensionSettingsPanelSections = extensionSettingsPanel
    ? extensionSettingsSections(extensionSettingsPanel.schema)
    : [];

  onMount(() => {
    loadSettingsBootstrap();
    loadExtensionsBootstrap();
  });

  onDestroy(() => {
    stopSystemAppearanceWatcher?.();
  });

  function applySettingsAppearanceTheme(theme: RuntimeSettings["appearance_theme"]) {
    stopSystemAppearanceWatcher?.();
    stopSystemAppearanceWatcher = configureAppearanceTheme(theme);
  }

  function loadSettingsBootstrap() {
    settingsLoading = true;
    paletteApi
      .getSettingsBootstrap()
      .then((bootstrap) => {
        settingsSaved = discardRuntimeSettingsDraft(bootstrap.config);
        settingsDraft = discardRuntimeSettingsDraft(bootstrap.config);
        defaultActivationShortcut = { ...bootstrap.default_activation_shortcut };
        settingsConfigPath = bootstrap.config_path;
        settingsConfigError = bootstrap.config_error;
        settingsMessage = null;
        settingsFailed = false;
        applySettingsAppearanceTheme(bootstrap.config.appearance_theme);
      })
      .catch((error: unknown) => {
        settingsMessage = errorMessage(error);
        settingsFailed = true;
      })
      .finally(() => {
        settingsLoading = false;
      });
  }

  function loadExtensionsBootstrap() {
    extensionsLoading = true;
    paletteApi
      .getExtensionsBootstrap()
      .then((bootstrap) => {
        extensionsBootstrap = bootstrap;
      })
      .catch((error: unknown) => {
        settingsMessage = errorMessage(error);
        settingsFailed = true;
      })
      .finally(() => {
        extensionsLoading = false;
      });
  }

  function updateSettingsDraft(update: (draft: RuntimeSettings) => void) {
    if (!settingsDraft) {
      return;
    }

    const next = discardRuntimeSettingsDraft(settingsDraft);
    update(next);
    settingsDraft = next;
  }

  function updateCommandBehavior(value: RuntimeSettings["command_behavior"]) {
    updateSettingsDraft((draft) => {
      draft.command_behavior = value;
    });
  }

  function updateAppearanceTheme(value: RuntimeSettings["appearance_theme"]) {
    updateSettingsDraft((draft) => {
      draft.appearance_theme = value;
    });
    applySettingsAppearanceTheme(value);
  }

  function updateCatalogEnabled(value: boolean) {
    updateSettingsDraft((draft) => {
      draft.github.enabled = value;
    });
  }

  function updateCatalogText(
    field: "owner" | "repo" | "branch" | "catalog_path",
    value: string,
  ) {
    updateSettingsDraft((draft) => {
      draft.github[field] = value;
    });
  }

  function recordActivationShortcut() {
    recordingActivationShortcut = true;
    settingsMessage = "Press the new activation shortcut";
    settingsFailed = false;
  }

  function resetActivationShortcut() {
    if (!defaultActivationShortcut) {
      return;
    }

    updateActivationShortcut(defaultActivationShortcut);
    recordingActivationShortcut = false;
    settingsMessage = `Reset to ${formatActivationShortcut(defaultActivationShortcut)}`;
    settingsFailed = false;
  }

  function updateActivationShortcut(shortcut: ActivationShortcut) {
    const nextShortcut = {
      ...shortcut,
      display_text: formatActivationShortcut(shortcut),
    };
    updateSettingsDraft((draft) => {
      draft.activation_shortcut = nextShortcut;
      draft.activation_hint = nextShortcut.display_text;
    });
  }

  function handleActivationShortcutKeydown(event: KeyboardEvent) {
    if (!recordingActivationShortcut) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    const shortcut = activationShortcutFromKeyboardEvent(event);
    if (!shortcut) {
      settingsMessage = "Press a supported non-modifier shortcut key";
      settingsFailed = false;
      return;
    }

    updateActivationShortcut(shortcut);
    recordingActivationShortcut = false;
    settingsMessage = `Recorded ${formatActivationShortcut(shortcut)}`;
    settingsFailed = false;
  }

  function saveRuntimeSettings() {
    if (!settingsSaved || !settingsDraft || settingsSaving || !settingsDirty) {
      return;
    }

    settingsSaving = true;
    paletteApi
      .saveRuntimeSettings(runtimeSettingsSaveRequestFromDraft(settingsDraft))
      .then(async (result) => {
        settingsConfigPath = result.runtime_status.config_path;
        settingsConfigError = result.runtime_status.config_error;

        if (!settingsSaved || !settingsDraft) {
          return;
        }

        const applied = applyRuntimeSettingsSaveResult(settingsSaved, settingsDraft, result);
        settingsSaved = applied.saved;
        settingsDraft = applied.draft;
        settingsMessage = applied.message;
        settingsFailed = applied.failed;
        applySettingsAppearanceTheme(applied.draft.appearance_theme);

        if (!applied.failed) {
          await reloadRuntimeStateAfterSave();
        }
      })
      .catch((error: unknown) => {
        settingsMessage = errorMessage(error);
        settingsFailed = true;
      })
      .finally(() => {
        settingsSaving = false;
      });
  }

  async function reloadRuntimeStateAfterSave() {
    try {
      const result = await paletteApi.reloadRuntimeState();
      if (result.status === "failed") {
        settingsMessage = `Settings saved; reload failed: ${result.message}`;
        settingsFailed = true;
      }
      await loadExtensionsBootstrap();
    } catch (error: unknown) {
      settingsMessage = `Settings saved; reload failed: ${errorMessage(error)}`;
      settingsFailed = true;
    }
  }

  function discardSettingsChanges() {
    if (!settingsSaved) {
      return;
    }

    settingsDraft = discardRuntimeSettingsDraft(settingsSaved);
    applySettingsAppearanceTheme(settingsDraft.appearance_theme);
    settingsMessage = "Changes discarded";
    settingsFailed = false;
  }

  function reloadRuntimeState() {
    if (settingsReloading) {
      return;
    }

    settingsReloading = true;
    paletteApi
      .reloadRuntimeState()
      .then(async (result) => {
        settingsMessage = result.message;
        settingsFailed = result.status === "failed";
        await loadExtensionsBootstrap();
      })
      .catch((error: unknown) => {
        settingsMessage = errorMessage(error);
        settingsFailed = true;
      })
      .finally(() => {
        settingsReloading = false;
      });
  }

  function showDebugOverlay() {
    paletteApi
      .showDebugOverlay()
      .then((status) => {
        settingsMessage = status.message;
        settingsFailed = status.status === "failed";
      })
      .catch((error: unknown) => {
        settingsMessage = errorMessage(error);
        settingsFailed = true;
      });
  }

  function refreshExtensionCatalog() {
    if (!settingsDraft || catalogRefreshing) {
      return;
    }

    catalogRefreshing = true;
    paletteApi
      .refreshExtensionCatalog(settingsDraft.github)
      .then((result) => {
        const applied = applyCatalogRefreshResult(catalogEntries, result);
        catalogEntries = applied.entries;
        settingsMessage = applied.message;
        settingsFailed = applied.failed;
      })
      .catch((error: unknown) => {
        settingsMessage = errorMessage(error);
        settingsFailed = true;
      })
      .finally(() => {
        catalogRefreshing = false;
      });
  }

  function installCatalogExtension(entry: CatalogEntry) {
    if (catalogInstallingId) {
      return;
    }

    catalogInstallingId = entry.id;
    paletteApi
      .installCatalogExtension(entry.id)
      .then((result) => {
        if (!extensionsBootstrap) {
          settingsMessage = result.message;
          settingsFailed = result.status === "failed";
          return;
        }

        const applied = applyExtensionMutationResult(extensionsBootstrap, result);
        extensionsBootstrap = applied.extensions;
        settingsMessage = applied.message;
        settingsFailed = applied.failed;
      })
      .catch((error: unknown) => {
        settingsMessage = errorMessage(error);
        settingsFailed = true;
      })
      .finally(() => {
        catalogInstallingId = null;
      });
  }

  function setExtensionEnabled(extension: ExtensionRow, enabled: boolean) {
    if (!extensionsBootstrap || extensionMutationKey) {
      return;
    }

    const mutationKey = extensionKey(extension);
    extensionMutationKey = mutationKey;
    paletteApi
      .setExtensionEnabled({
        extension_id: extension.id,
        source_id: extension.source_id,
        enabled,
      })
      .then((result) => {
        if (!extensionsBootstrap) {
          return;
        }

        const applied = applyExtensionMutationResult(extensionsBootstrap, result);
        extensionsBootstrap = applied.extensions;
        settingsMessage = applied.message;
        settingsFailed = applied.failed;
      })
      .catch((error: unknown) => {
        settingsMessage = errorMessage(error);
        settingsFailed = true;
      })
      .finally(() => {
        extensionMutationKey = null;
      });
  }

  function uninstallExtension(extension: ExtensionRow) {
    if (!extensionsBootstrap || extensionMutationKey || !extension.can_uninstall) {
      return;
    }

    const mutationKey = extensionKey(extension);
    extensionMutationKey = mutationKey;
    paletteApi
      .uninstallExtension({
        extension_id: extension.id,
        source_id: extension.source_id,
      })
      .then((result) => {
        if (!extensionsBootstrap) {
          return;
        }

        const applied = applyExtensionMutationResult(extensionsBootstrap, result);
        extensionsBootstrap = applied.extensions;
        settingsMessage = applied.message;
        settingsFailed = applied.failed;
      })
      .catch((error: unknown) => {
        settingsMessage = errorMessage(error);
        settingsFailed = true;
      })
      .finally(() => {
        extensionMutationKey = null;
      });
  }

  function openExtensionSettings(extension: ExtensionRow) {
    if (!extension.has_settings || extensionSettingsLoadingKey) {
      return;
    }

    const mutationKey = extensionKey(extension);
    extensionSettingsLoadingKey = mutationKey;
    paletteApi
      .getExtensionSettings({
        extension_id: extension.id,
        source_id: extension.source_id,
      })
      .then((result) => {
        if (result.status === "failed" || !result.target || !result.schema) {
          settingsMessage = result.message;
          settingsFailed = true;
          return;
        }

        extensionSettingsPanel = {
          target: result.target,
          schema: result.schema,
          saved: copyExtensionSettingsValues(result.values),
          draft: copyExtensionSettingsValues(result.values),
          saving: false,
          message: result.message,
          failed: false,
        };
        settingsMessage = result.message;
        settingsFailed = false;
      })
      .catch((error: unknown) => {
        settingsMessage = errorMessage(error);
        settingsFailed = true;
      })
      .finally(() => {
        extensionSettingsLoadingKey = null;
      });
  }

  function closeExtensionSettingsPanel() {
    extensionSettingsPanel = null;
  }

  function resetExtensionSettingsDefaults() {
    if (!extensionSettingsPanel) {
      return;
    }

    extensionSettingsPanel = {
      ...extensionSettingsPanel,
      draft: defaultExtensionSettingsValues(extensionSettingsPanel.schema),
      message: "Defaults restored",
      failed: false,
    };
  }

  function updateExtensionSettingsDraft(
    update: (draft: ExtensionSettingsValues) => ExtensionSettingsValues,
  ) {
    if (!extensionSettingsPanel) {
      return;
    }

    extensionSettingsPanel = {
      ...extensionSettingsPanel,
      draft: update(extensionSettingsPanel.draft),
      message: null,
      failed: false,
    };
  }

  function setExtensionSettingToggle(key: string, enabled: boolean) {
    updateExtensionSettingsDraft((draft) => updateExtensionSettingToggle(draft, key, enabled));
  }

  function addExtensionSettingEntry(item: ExtensionSettingItem) {
    updateExtensionSettingsDraft((draft) => addExtensionSettingListEntry(draft, item));
  }

  function updateExtensionSettingEntry(
    key: string,
    index: number,
    patch: { name?: string; format?: string; enabled?: boolean },
  ) {
    updateExtensionSettingsDraft((draft) =>
      updateExtensionSettingListEntry(draft, key, index, patch),
    );
  }

  function removeExtensionSettingEntry(key: string, index: number) {
    updateExtensionSettingsDraft((draft) => removeExtensionSettingListEntry(draft, key, index));
  }

  function saveExtensionSettingsPanel() {
    if (!extensionSettingsPanel || extensionSettingsPanel.saving || !extensionSettingsDirty) {
      return;
    }

    const panel = extensionSettingsPanel;
    extensionSettingsPanel = {
      ...panel,
      saving: true,
      message: null,
      failed: false,
    };
    paletteApi
      .saveExtensionSettings(extensionSettingsSaveRequestFromDraft(panel.target, panel.draft))
      .then((result) => {
        if (!extensionSettingsPanel) {
          return;
        }

        const applied = applyExtensionSettingsSaveResult(
          extensionSettingsPanel.saved,
          extensionSettingsPanel.draft,
          result,
        );
        extensionSettingsPanel = {
          ...extensionSettingsPanel,
          saved: applied.saved,
          draft: applied.draft,
          saving: false,
          message: applied.message,
          failed: applied.failed,
        };
        settingsMessage = applied.message;
        settingsFailed = applied.failed;
      })
      .catch((error: unknown) => {
        if (extensionSettingsPanel) {
          extensionSettingsPanel = {
            ...extensionSettingsPanel,
            saving: false,
            message: errorMessage(error),
            failed: true,
          };
        }
        settingsMessage = errorMessage(error);
        settingsFailed = true;
      });
  }

  function categoryToggleItem(section: ExtensionSettingsSection): ExtensionSettingItem | null {
    if (!section.category.toggle_key) {
      return null;
    }

    return section.items.find((item) => item.key === section.category.toggle_key) ?? null;
  }

  function categoryToggleItems(section: ExtensionSettingsSection): ExtensionSettingItem[] {
    const item = categoryToggleItem(section);
    return item ? [item] : [];
  }

  function visibleSectionItems(section: ExtensionSettingsSection): ExtensionSettingItem[] {
    return section.items.filter((item) => item.key !== section.category.toggle_key);
  }

  function extensionSettingToggleValue(
    values: ExtensionSettingsValues,
    item: ExtensionSettingItem,
  ): boolean {
    return values.toggles[item.key] ?? item.default;
  }

  function extensionSettingListValue(
    values: ExtensionSettingsValues,
    item: ExtensionSettingItem,
  ) {
    return values.lists[item.key] ?? item.default_entries;
  }

  function extensionKey(extension: ExtensionRow): string {
    return `${extension.source_id}/${extension.id}`;
  }

  function extensionKindLabel(extension: ExtensionRow): string {
    return extension.kind === "wasm_plugin" ? "Plugin" : "Static";
  }

  function extensionStatusLabel(extension: ExtensionRow): string {
    return extension.enabled ? "Enabled" : "Disabled";
  }

  function extensionStatusReservedLabel(): string {
    return EXTENSION_STATUS_RESERVED_LABEL;
  }

  function extensionStatusPillClass(extension: ExtensionRow): string {
    return [
      "extension-status-pill rounded border px-3 py-1 text-xs font-medium",
      extension.enabled ? "settings-status-pill-enabled" : "settings-status-pill-disabled",
    ].join(" ");
  }

  function installedVersionForCatalogEntry(
    entry: CatalogEntry,
    bootstrap: ExtensionsBootstrap | null,
  ): string | null {
    return (
      bootstrap?.downloaded_extensions.find(
        (extension) => extension.id === entry.id && extension.source_id === "github",
      )?.version ?? null
    );
  }

  function catalogActionLabel(
    entry: CatalogEntry,
    bootstrap: ExtensionsBootstrap | null,
  ): string {
    const installedVersion = installedVersionForCatalogEntry(entry, bootstrap);
    if (!installedVersion) {
      return "Install";
    }

    return installedVersion === entry.version ? "Reinstall" : "Update";
  }

  function catalogActionReservedLabel(): string {
    return CATALOG_ACTION_RESERVED_LABEL;
  }

  function catalogStatusLabel(
    entry: CatalogEntry,
    bootstrap: ExtensionsBootstrap | null,
  ): string | null {
    const installedVersion = installedVersionForCatalogEntry(entry, bootstrap);
    if (!installedVersion) {
      return null;
    }

    return installedVersion === entry.version ? "Installed" : "Update available";
  }

  function inputValue(event: Event): string {
    return (event.currentTarget as HTMLInputElement).value;
  }

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }
</script>

<svelte:window onkeydown={handleActivationShortcutKeydown} />

<main class="settings-shell">
  <section class="flex min-h-screen">
    <aside class="settings-sidebar w-56 p-4">
      <h1 class="text-lg font-semibold">Omni Palette</h1>
      <p class="settings-muted text-sm">Preferences</p>
      <nav class="mt-6 grid gap-2">
        <button
          class={[
            "rounded border px-3 py-2 text-left text-sm",
            activeSettingsPage === "general"
              ? "border-[var(--settings-accent)] bg-[var(--settings-surface-alt)] text-[var(--settings-text-primary)]"
              : "border-transparent text-[var(--settings-text-secondary)]",
          ].join(" ")}
          onclick={() => (activeSettingsPage = "general")}
          type="button"
        >
          <span class="block font-medium">General</span>
          <span class="settings-nav-description block text-xs">Shortcut and config</span>
        </button>
        <button
          class={[
            "rounded border px-3 py-2 text-left text-sm",
            activeSettingsPage === "extensions"
              ? "border-[var(--settings-accent)] bg-[var(--settings-surface-alt)] text-[var(--settings-text-primary)]"
              : "border-transparent text-[var(--settings-text-secondary)]",
          ].join(" ")}
          onclick={() => (activeSettingsPage = "extensions")}
          type="button"
        >
          <span class="block font-medium">Manage Extensions</span>
          <span class="settings-nav-description block text-xs">Enable and remove</span>
        </button>
        <button
          class={[
            "rounded border px-3 py-2 text-left text-sm",
            activeSettingsPage === "marketplace"
              ? "border-[var(--settings-accent)] bg-[var(--settings-surface-alt)] text-[var(--settings-text-primary)]"
              : "border-transparent text-[var(--settings-text-secondary)]",
          ].join(" ")}
          onclick={() => (activeSettingsPage = "marketplace")}
          type="button"
        >
          <span class="block font-medium">Marketplace</span>
          <span class="settings-nav-description block text-xs">Browse and install</span>
        </button>
      </nav>
    </aside>

    <div class="flex-1 p-6">
      <header class="mb-6 flex items-start justify-between gap-4">
        <div>
          {#if activeSettingsPage === "general"}
            <h2 class="text-2xl font-semibold">General</h2>
            <p class="mt-1 text-sm text-[var(--settings-text-muted)]">
              Control how Omni Palette opens and where preferences are stored.
            </p>
          {:else if activeSettingsPage === "extensions"}
            <h2 class="text-2xl font-semibold">Installed Extensions</h2>
            <p class="mt-1 text-sm text-[var(--settings-text-muted)]">
              Manage extensions that are available on this device.
            </p>
          {:else}
            <h2 class="text-2xl font-semibold">Extension Marketplace</h2>
            <p class="mt-1 text-sm text-[var(--settings-text-muted)]">
              Configure the catalog source for future extension installs.
            </p>
          {/if}
        </div>
        <button
          class="settings-button disabled:text-[var(--settings-text-muted)]"
          disabled={settingsReloading}
          onclick={reloadRuntimeState}
          type="button"
        >
          <span class="settings-stable-label">
            <span aria-hidden="true" class="settings-stable-label-reserved">
              {HEADER_RELOAD_RESERVED_LABEL}
            </span>
            <span class="settings-stable-label-visible">
              {settingsReloading ? "Reloading..." : "Reload extensions"}
            </span>
          </span>
        </button>
      </header>

      {#if settingsConfigError}
        <p class="settings-error-message mb-4 px-3 py-2 text-sm">
          {settingsConfigError}
        </p>
      {/if}
      {#if extensionsBootstrap?.install_root_error}
        <p class="settings-error-message mb-4 px-3 py-2 text-sm">
          {extensionsBootstrap.install_root_error}
        </p>
      {/if}
      {#if settingsMessage}
        <p
          class={[
            "mb-4 px-3 py-2 text-sm",
            settingsFailed ? "settings-error-message" : "settings-success-message",
          ].join(" ")}
        >
          {settingsMessage}
        </p>
      {/if}

      {#if settingsLoading || !settingsDraft}
        <div class="settings-card settings-muted p-6 text-sm">
          Loading settings...
        </div>
      {:else}
        {#if activeSettingsPage === "general"}
          <div class="space-y-6">
            <section class="settings-section">
              <h3 class="settings-section-title">Appearance</h3>
              <div class="settings-row">
                <div>
                  <p class="settings-row-label">Theme</p>
                  <p class="settings-row-description">
                    Follow Windows or use a fixed light or dark theme.
                  </p>
                </div>
                <div class="settings-row-control">
                  {#each ["system", "light", "dark"] as theme}
                    <label
                      class={[
                        "settings-segmented-option capitalize",
                        settingsDraft.appearance_theme === theme
                          ? "settings-segmented-option-selected"
                          : "",
                      ].join(" ")}
                    >
                      <input
                        checked={settingsDraft.appearance_theme === theme}
                        class="sr-only"
                        name="appearance-theme"
                        onchange={() =>
                          updateAppearanceTheme(theme as RuntimeSettings["appearance_theme"])}
                        type="radio"
                      />
                      {theme}
                    </label>
                  {/each}
                </div>
              </div>
            </section>

            <section class="settings-section">
              <h3 class="settings-section-title">Activation</h3>
              <div class="settings-row">
                <div>
                  <p class="settings-row-label">Activation shortcut</p>
                  <p class="settings-row-description">
                    The global shortcut that opens Omni Palette.
                  </p>
                </div>
                <div class="settings-row-control">
                  <span class="settings-chip">
                    {formatActivationShortcut(settingsDraft.activation_shortcut)}
                  </span>
                  <button
                    class="settings-button"
                    disabled={recordingActivationShortcut}
                    onclick={recordActivationShortcut}
                    type="button"
                  >
                    <span class="settings-stable-label">
                      <span aria-hidden="true" class="settings-stable-label-reserved">
                        {RECORD_SHORTCUT_RESERVED_LABEL}
                      </span>
                      <span class="settings-stable-label-visible">
                        {recordingActivationShortcut ? "Recording..." : "Record"}
                      </span>
                    </span>
                  </button>
                  <button
                    class="settings-button"
                    disabled={!defaultActivationShortcut}
                    onclick={resetActivationShortcut}
                    type="button"
                  >
                    Reset
                  </button>
                </div>
              </div>
            </section>

            <section class="settings-section">
              <h3 class="settings-section-title">Command behavior</h3>
              <div class="settings-row">
                <div>
                  <p class="settings-row-label">Mode</p>
                  <p class="settings-row-description">
                    Execute commands immediately or show their guide first.
                  </p>
                </div>
                <div class="settings-row-control">
                  <label
                    class={[
                      "settings-segmented-option",
                      settingsDraft.command_behavior === "execute"
                        ? "settings-segmented-option-selected"
                        : "",
                    ].join(" ")}
                  >
                    <input
                      checked={settingsDraft.command_behavior === "execute"}
                      class="sr-only"
                      name="command-behavior"
                      onchange={() => updateCommandBehavior("execute")}
                      type="radio"
                    />
                    Execute
                  </label>
                  <label
                    class={[
                      "settings-segmented-option",
                      settingsDraft.command_behavior === "guide"
                        ? "settings-segmented-option-selected"
                        : "",
                    ].join(" ")}
                  >
                    <input
                      checked={settingsDraft.command_behavior === "guide"}
                      class="sr-only"
                      name="command-behavior"
                      onchange={() => updateCommandBehavior("guide")}
                      type="radio"
                    />
                    Guide
                  </label>
                </div>
              </div>
            </section>

            <section class="settings-section">
              <h3 class="settings-section-title">Debug</h3>
              <div class="settings-row">
                <div>
                  <p class="settings-row-label">Context overlay</p>
                  <p class="settings-row-description">
                    Inspect the active context used for filtering commands.
                  </p>
                </div>
                <div class="settings-row-control">
                  <button
                    class="settings-button"
                    onclick={showDebugOverlay}
                    type="button"
                  >
                    Pop up debugger
                  </button>
                </div>
              </div>
            </section>

            <section class="settings-section">
              <h3 class="settings-section-title">Storage</h3>
              <div class="settings-row">
                <div>
                  <p class="settings-row-label">User config</p>
                  <p class="settings-row-description">
                    Runtime preferences are saved as TOML.
                  </p>
                </div>
                <div class="settings-row-control">
                  <span class="settings-chip settings-path-chip">
                    {settingsConfigPath ?? "Config path unavailable"}
                  </span>
                </div>
              </div>
            </section>
          </div>
        {:else if activeSettingsPage === "extensions"}
          {#if extensionsLoading || !extensionsBootstrap}
            <div class="settings-card settings-muted p-6 text-sm">
              Loading extensions...
            </div>
          {:else}
            <div class="space-y-6">
              <section class="settings-card p-4">
                <h3 class="text-lg font-medium">Bundled Defaults</h3>
                <p class="settings-muted text-sm">
                  Built into Omni Palette. They can be disabled, but not uninstalled.
                </p>
                <div class="mt-4 grid gap-3">
                  {#each extensionsBootstrap.bundled_extensions as extension}
                    <article class="settings-subcard p-4">
                      <div class="flex flex-wrap items-center justify-between gap-3">
                        <div>
                          <h4 class="font-medium">
                            {extension.name}
                            <span class="settings-muted text-xs">{extension.version}</span>
                          </h4>
                          <div class="settings-muted mt-1 flex flex-wrap gap-2 text-xs">
                            <span>Bundled</span>
                            <span>{extensionKindLabel(extension)}</span>
                          </div>
                        </div>
                        <div class="flex flex-wrap items-center gap-2">
                          <span class={extensionStatusPillClass(extension)}>
                            <span class="settings-stable-label">
                              <span
                                aria-hidden="true"
                                class="settings-stable-label-reserved"
                              >
                                {extensionStatusReservedLabel()}
                              </span>
                              <span class="settings-stable-label-visible">
                                {extensionStatusLabel(extension)}
                              </span>
                            </span>
                          </span>
                          <ToggleSwitch
                            ariaLabel={`Toggle ${extension.name}`}
                            checked={extension.enabled}
                            disabled={extensionMutationKey === extensionKey(extension)}
                            onToggle={(enabled) => setExtensionEnabled(extension, enabled)}
                            updating={extensionMutationKey === extensionKey(extension)}
                          />
                          {#if extension.has_settings}
                            <button
                              class="settings-button"
                              disabled={extensionSettingsLoadingKey === extensionKey(extension)}
                              onclick={() => openExtensionSettings(extension)}
                              type="button"
                            >
                              <span class="settings-stable-label">
                                <span
                                  aria-hidden="true"
                                  class="settings-stable-label-reserved"
                                >
                                  {EXTENSION_SETTINGS_RESERVED_LABEL}
                                </span>
                                <span class="settings-stable-label-visible">
                                  {extensionSettingsLoadingKey === extensionKey(extension)
                                    ? "Loading..."
                                    : "Settings"}
                                </span>
                              </span>
                            </button>
                          {/if}
                        </div>
                      </div>
                    </article>
                  {/each}
                </div>
              </section>

              <section class="settings-card p-4">
                <h3 class="text-lg font-medium">Downloaded Extensions</h3>
                <p class="settings-muted text-sm">Installed from your configured catalog.</p>
                {#if extensionsBootstrap.downloaded_extensions.length === 0}
                  <p class="settings-empty mt-4 px-3 py-4 text-sm">
                    No downloaded extensions installed yet.
                  </p>
                {:else}
                  <div class="mt-4 grid gap-3">
                    {#each extensionsBootstrap.downloaded_extensions as extension}
                      <article class="settings-subcard p-4">
                        <div class="flex flex-wrap items-center justify-between gap-3">
                          <div>
                            <h4 class="font-medium">
                              {extension.name}
                              <span class="settings-muted text-xs">{extension.version}</span>
                            </h4>
                            <div class="settings-muted mt-1 flex flex-wrap gap-2 text-xs">
                              <span>Downloaded</span>
                              <span>{extensionKindLabel(extension)}</span>
                            </div>
                          </div>
                          <div class="flex flex-wrap items-center gap-2">
                            <span class={extensionStatusPillClass(extension)}>
                              <span class="settings-stable-label">
                                <span
                                  aria-hidden="true"
                                  class="settings-stable-label-reserved"
                                >
                                  {extensionStatusReservedLabel()}
                                </span>
                                <span class="settings-stable-label-visible">
                                  {extensionStatusLabel(extension)}
                                </span>
                              </span>
                            </span>
                            <ToggleSwitch
                              ariaLabel={`Toggle ${extension.name}`}
                              checked={extension.enabled}
                              disabled={extensionMutationKey === extensionKey(extension)}
                              onToggle={(enabled) => setExtensionEnabled(extension, enabled)}
                              updating={extensionMutationKey === extensionKey(extension)}
                            />
                            {#if extension.has_settings}
                              <button
                                class="settings-button"
                                disabled={extensionSettingsLoadingKey === extensionKey(extension)}
                                onclick={() => openExtensionSettings(extension)}
                                type="button"
                              >
                                <span class="settings-stable-label">
                                  <span
                                    aria-hidden="true"
                                    class="settings-stable-label-reserved"
                                  >
                                    {EXTENSION_SETTINGS_RESERVED_LABEL}
                                  </span>
                                  <span class="settings-stable-label-visible">
                                    {extensionSettingsLoadingKey === extensionKey(extension)
                                      ? "Loading..."
                                      : "Settings"}
                                  </span>
                                </span>
                              </button>
                            {/if}
                            <button
                              class="settings-danger-button"
                              disabled={extensionMutationKey === extensionKey(extension)}
                              onclick={() => uninstallExtension(extension)}
                              type="button"
                            >
                              Uninstall
                            </button>
                          </div>
                        </div>
                      </article>
                    {/each}
                  </div>
                {/if}
              </section>
            </div>
          {/if}
        {:else}
          <div class="space-y-6">
            <fieldset class="settings-card p-4">
              <legend class="settings-section-heading px-1 text-sm font-medium">Catalog source</legend>
              <div class="settings-label flex items-center gap-3 text-sm">
                <ToggleSwitch
                  ariaLabel="Enable GitHub catalog source"
                  checked={settingsDraft.github.enabled}
                  onToggle={updateCatalogEnabled}
                />
                <span>Enable GitHub catalog source</span>
              </div>
              <div class="mt-4 grid gap-3 sm:grid-cols-2">
                <label class="settings-label grid gap-1 text-sm">
                  Owner
                  <input
                    class="settings-input px-3 py-2"
                    value={settingsDraft.github.owner}
                    oninput={(event) => updateCatalogText("owner", inputValue(event))}
                  />
                </label>
                <label class="settings-label grid gap-1 text-sm">
                  Repo
                  <input
                    class="settings-input px-3 py-2"
                    value={settingsDraft.github.repo}
                    oninput={(event) => updateCatalogText("repo", inputValue(event))}
                  />
                </label>
                <label class="settings-label grid gap-1 text-sm">
                  Branch
                  <input
                    class="settings-input px-3 py-2"
                    value={settingsDraft.github.branch}
                    oninput={(event) => updateCatalogText("branch", inputValue(event))}
                  />
                </label>
                <label class="settings-label grid gap-1 text-sm">
                  Catalog path
                  <input
                    class="settings-input px-3 py-2"
                    value={settingsDraft.github.catalog_path}
                    oninput={(event) => updateCatalogText("catalog_path", inputValue(event))}
                  />
                </label>
              </div>
              <div class="mt-4 flex flex-wrap gap-2">
                <button
                  class="settings-primary-button"
                  disabled={!settingsDirty || settingsSaving}
                  onclick={saveRuntimeSettings}
                  type="button"
                >
                  <span class="settings-stable-label">
                    <span aria-hidden="true" class="settings-stable-label-reserved">
                      {SAVE_SOURCE_RESERVED_LABEL}
                    </span>
                    <span class="settings-stable-label-visible">
                      {settingsSaving ? "Saving..." : "Save Source"}
                    </span>
                  </span>
                </button>
                <button
                  class="settings-button"
                  disabled={!settingsDraft.github.enabled || catalogRefreshing}
                  onclick={refreshExtensionCatalog}
                  type="button"
                >
                  <span class="settings-stable-label">
                    <span aria-hidden="true" class="settings-stable-label-reserved">
                      {REFRESH_CATALOG_RESERVED_LABEL}
                    </span>
                    <span class="settings-stable-label-visible">
                      {catalogRefreshing ? "Refreshing..." : "Refresh Catalog"}
                    </span>
                  </span>
                </button>
                <button
                  class="settings-button"
                  disabled={settingsReloading}
                  onclick={reloadRuntimeState}
                  type="button"
                >
                  <span class="settings-stable-label">
                    <span aria-hidden="true" class="settings-stable-label-reserved">
                      {CATALOG_RELOAD_RESERVED_LABEL}
                    </span>
                    <span class="settings-stable-label-visible">
                      {settingsReloading ? "Reloading..." : "Reload Extensions"}
                    </span>
                  </span>
                </button>
              </div>
            </fieldset>

            <section class="settings-card p-4">
              <h3 class="text-lg font-medium">Available Extensions</h3>
              <p class="settings-muted text-sm">
                Search the refreshed catalog for extensions that support this Windows build.
              </p>
              <input
                class="settings-input mt-4 w-full max-w-lg px-3 py-2"
                placeholder="Search catalog"
                value={catalogQuery}
                oninput={(event) => (catalogQuery = inputValue(event))}
              />

              {#if catalogEntries.length === 0}
                <p class="settings-empty mt-4 px-3 py-4 text-sm">
                  Refresh the catalog to browse available extensions.
                </p>
              {:else if visibleCatalogEntries.length === 0}
                <p class="settings-empty mt-4 px-3 py-4 text-sm">
                  No catalog extensions match your search.
                </p>
              {:else}
                <div class="mt-4 grid gap-3">
                  {#each visibleCatalogEntries as entry}
                    <article class="settings-subcard p-4">
                      <div class="flex flex-wrap items-center justify-between gap-3">
                        <div>
                          <h4 class="font-medium">
                            {entry.name}
                            <span class="settings-muted text-xs">{entry.version}</span>
                          </h4>
                          {#if entry.description}
                            <p class="settings-muted mt-1 text-sm">{entry.description}</p>
                          {/if}
                          {#if catalogStatusLabel(entry, extensionsBootstrap)}
                            <p class="mt-1 text-xs text-[var(--settings-accent)]">
                              {catalogStatusLabel(entry, extensionsBootstrap)}
                            </p>
                          {/if}
                        </div>
                        <div class="flex flex-wrap items-center gap-2">
                          {#if entry.kind === "static"}
                            <button
                              class="settings-primary-button"
                              disabled={catalogInstallingId === entry.id}
                              onclick={() => installCatalogExtension(entry)}
                              type="button"
                            >
                              <span class="settings-stable-label">
                                <span
                                  aria-hidden="true"
                                  class="settings-stable-label-reserved"
                                >
                                  {catalogActionReservedLabel()}
                                </span>
                                <span class="settings-stable-label-visible">
                                  {catalogInstallingId === entry.id
                                    ? "Installing..."
                                    : catalogActionLabel(entry, extensionsBootstrap)}
                                </span>
                              </span>
                            </button>
                          {:else}
                            <span class="settings-muted text-sm">Unavailable</span>
                          {/if}
                        </div>
                      </div>
                    </article>
                  {/each}
                </div>
              {/if}
            </section>
          </div>
        {/if}

        {#if activeSettingsPage !== "extensions"}
          <div class="mt-6 flex items-center justify-between gap-3 border-t border-[var(--settings-border-soft)] pt-4">
            <span class="text-sm text-[var(--settings-text-muted)]">
              {settingsDirty ? "Unsaved changes" : "Settings are current"}
            </span>
            <div class="flex gap-2">
              <button
                class="settings-button"
                disabled={!settingsDirty || settingsSaving}
                onclick={discardSettingsChanges}
                type="button"
              >
                Discard
              </button>
              <button
                class="settings-primary-button"
                disabled={!settingsDirty || settingsSaving}
                onclick={saveRuntimeSettings}
                type="button"
              >
                <span class="settings-stable-label">
                  <span aria-hidden="true" class="settings-stable-label-reserved">
                    {SAVE_SETTINGS_RESERVED_LABEL}
                  </span>
                  <span class="settings-stable-label-visible">
                    {settingsSaving ? "Saving..." : "Save settings"}
                  </span>
                </span>
              </button>
            </div>
          </div>
        {/if}
      {/if}
    </div>
  </section>

  {#if extensionSettingsPanel}
    <section class="settings-modal-backdrop fixed inset-0 z-10 flex items-center justify-center p-6">
      <div class="settings-modal-panel max-h-full w-full max-w-3xl overflow-auto p-5">
        <header class="flex items-start justify-between gap-4">
          <div>
            <h2 class="text-xl font-semibold">
              {extensionSettingsPanel.target.display_name} Settings
            </h2>
            <p class="settings-muted mt-1 text-sm">
              These settings affect only this extension and are saved in your user extension folder.
            </p>
          </div>
          <button
            class="settings-button"
            disabled={extensionSettingsPanel.saving}
            onclick={closeExtensionSettingsPanel}
            type="button"
          >
            Close
          </button>
        </header>

        {#if extensionSettingsPanel.message}
          <p
            class={[
              "mt-4 px-3 py-2 text-sm",
              extensionSettingsPanel.failed
                ? "settings-error-message"
                : "settings-success-message",
            ].join(" ")}
          >
            {extensionSettingsPanel.message}
          </p>
        {/if}

        {#if extensionSettingsPanel.schema.items.length === 0}
          <p class="settings-empty mt-4 px-3 py-4 text-sm">
            No custom settings are currently available for this extension.
          </p>
        {:else}
          <div class="mt-4 grid gap-4">
            {#each extensionSettingsPanelSections as section}
              <section class="settings-card p-4">
                <div class="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <h3 class="font-medium">{section.category.label}</h3>
                    {#if section.category.description}
                      <p class="settings-muted mt-1 text-sm">{section.category.description}</p>
                    {/if}
                  </div>
                  {#each categoryToggleItems(section) as toggleItem}
                    <div class="settings-label flex items-center gap-2 text-sm">
                      <ToggleSwitch
                        ariaLabel={`Toggle ${toggleItem.label}`}
                        checked={extensionSettingToggleValue(extensionSettingsPanel.draft, toggleItem)}
                        onToggle={(enabled) =>
                          setExtensionSettingToggle(toggleItem.key, enabled)}
                      />
                      <span>{toggleItem.label}</span>
                    </div>
                  {/each}
                </div>

                <div class="mt-4 grid gap-3">
                  {#each visibleSectionItems(section) as item}
                    {#if item.kind === "toggle"}
                      <div class="settings-subcard flex items-start justify-between gap-3 p-3 text-sm">
                        <span>
                          <span class="settings-label block">{item.label}</span>
                          {#if item.description}
                            <span class="settings-muted mt-1 block">{item.description}</span>
                          {/if}
                        </span>
                        <ToggleSwitch
                          ariaLabel={`Toggle ${item.label}`}
                          checked={extensionSettingToggleValue(extensionSettingsPanel.draft, item)}
                          onToggle={(enabled) => setExtensionSettingToggle(item.key, enabled)}
                        />
                      </div>
                    {:else}
                      <div class="settings-subcard p-3">
                        <div class="flex flex-wrap items-start justify-between gap-3">
                          <div>
                            <h4 class="text-sm font-medium">{item.label}</h4>
                            {#if item.description}
                              <p class="settings-muted mt-1 text-sm">{item.description}</p>
                            {/if}
                          </div>
                          <button
                            class="settings-button"
                            onclick={() => addExtensionSettingEntry(item)}
                            type="button"
                          >
                            Add Entry
                          </button>
                        </div>

                        <div class="mt-3 grid gap-2">
                          {#each extensionSettingListValue(extensionSettingsPanel.draft, item) as entry, index (entry.id)}
                            <div class="settings-card grid gap-2 p-3 md:grid-cols-[auto_1fr_1fr_auto]">
                              <div class="settings-label flex items-center gap-2 text-sm">
                                <ToggleSwitch
                                  ariaLabel={`Toggle ${entry.name || "entry"} enabled`}
                                  checked={entry.enabled}
                                  onToggle={(enabled) =>
                                    updateExtensionSettingEntry(item.key, index, {
                                      enabled,
                                    })}
                                />
                                <span>Enabled</span>
                              </div>
                              <input
                                class="settings-input px-3 py-2 text-sm"
                                placeholder="Name"
                                value={entry.name}
                                oninput={(event) =>
                                  updateExtensionSettingEntry(item.key, index, {
                                    name: inputValue(event),
                                  })}
                              />
                              <input
                                class="settings-input px-3 py-2 text-sm"
                                placeholder={item.entry_list_format_hint ?? "Format"}
                                value={entry.format}
                                oninput={(event) =>
                                  updateExtensionSettingEntry(item.key, index, {
                                    format: inputValue(event),
                                  })}
                              />
                              <button
                                class="settings-button"
                                onclick={() => removeExtensionSettingEntry(item.key, index)}
                                type="button"
                              >
                                Remove
                              </button>
                            </div>
                          {/each}
                        </div>
                      </div>
                    {/if}
                  {/each}
                </div>
              </section>
            {/each}
          </div>
        {/if}

        <footer class="settings-modal-footer mt-5 flex flex-wrap items-center justify-between gap-3 pt-4">
          <span class="settings-muted text-sm">
            {extensionSettingsDirty ? "Unsaved extension settings" : "Extension settings are current"}
          </span>
          <div class="flex flex-wrap gap-2">
            <button
              class="settings-button"
              disabled={extensionSettingsPanel.saving}
              onclick={resetExtensionSettingsDefaults}
              type="button"
            >
              Reset Defaults
            </button>
            <button
              class="settings-primary-button"
              disabled={!extensionSettingsDirty || extensionSettingsPanel.saving}
              onclick={saveExtensionSettingsPanel}
              type="button"
            >
              <span class="settings-stable-label">
                <span aria-hidden="true" class="settings-stable-label-reserved">
                  {SAVE_EXTENSION_SETTINGS_RESERVED_LABEL}
                </span>
                <span class="settings-stable-label-visible">
                  {extensionSettingsPanel.saving ? "Saving..." : "Save Settings"}
                </span>
              </span>
            </button>
          </div>
        </footer>
      </div>
    </section>
  {/if}
</main>
