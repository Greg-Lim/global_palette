<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  import type { GuideEventPayload, GuideStatus } from "./commands";
  import {
    GUIDE_EVENT_NAME,
    configureRuntimeAppearanceTheme,
    guideShortcutParts,
    nextGuideStatus,
    paletteApi,
  } from "./commands";

  let guideStatus: GuideStatus | null = null;
  let error: string | null = null;

  $: shortcutParts = guideShortcutParts(guideStatus?.shortcut_text ?? "");
  $: fallbackText = `${guideStatus?.activation_hint ?? "Ctrl+Shift+P"} to run for me`;

  onMount(() => {
    let mounted = true;
    let stopAppearanceTheme: (() => void) | null = null;

    configureRuntimeAppearanceTheme(paletteApi)
      .then((cleanup) => {
        if (mounted) {
          stopAppearanceTheme = cleanup;
        } else {
          cleanup();
        }
      })
      .catch((caught: unknown) => {
        console.warn("Failed to apply appearance theme", caught);
      });

    paletteApi
      .getGuideStatus()
      .then((status) => {
        guideStatus = status;
      })
      .catch((caught: unknown) => {
        error = errorMessage(caught);
      });

    let unlistenGuideEvents: (() => void) | null = null;
    listen<GuideEventPayload>(GUIDE_EVENT_NAME, (event) => {
      guideStatus = nextGuideStatus(guideStatus, event.payload);
    })
      .then((unlisten) => {
        unlistenGuideEvents = unlisten;
      })
      .catch((caught: unknown) => {
        error = errorMessage(caught);
      });

    return () => {
      mounted = false;
      stopAppearanceTheme?.();
      unlistenGuideEvents?.();
    };
  });

  function cancelGuide() {
    paletteApi
      .cancelGuide()
      .then((status) => {
        guideStatus = status;
      })
      .catch((caught: unknown) => {
        error = errorMessage(caught);
      });
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      cancelGuide();
    }
  }

  function errorMessage(caught: unknown): string {
    return caught instanceof Error ? caught.message : String(caught);
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<main class="guide-shell flex h-screen overflow-hidden items-center justify-center p-4">
  <section class="guide-panel w-full px-6 py-5">
    {#if error}
      <p class="guide-error text-sm">{error}</p>
    {:else if guideStatus?.active}
      <p class="text-sm font-semibold">{guideStatus.command_label}</p>

      {#if shortcutParts.length > 0}
        <div class="mt-4 flex flex-wrap items-center gap-3">
          {#each shortcutParts as chord, chordIndex}
            {#if chordIndex > 0}
              <span class="guide-sequence-separator text-xs">then</span>
            {/if}
            <span class="flex items-center gap-2">
              {#each chord as key}
                <kbd class="guide-keycap min-h-24 min-w-32 rounded-md px-8 py-6 text-center text-2xl font-semibold">
                  {key}
                </kbd>
              {/each}
            </span>
          {/each}
        </div>
      {/if}

      <p class="guide-muted mt-4 text-xs">{fallbackText}</p>
    {:else}
      <p class="guide-muted text-sm">Guide idle</p>
    {/if}
  </section>
</main>
