<script lang="ts">
  import { onMount } from "svelte";

  import type { DebugCommandRow, DebugOverlayStatus, DebugSnapshot } from "./commands";
  import { formatDebugOverlayStatus, paletteApi } from "./commands";

  const DEBUG_SNAPSHOT_REFRESH_MS = 1000;

  let status: DebugOverlayStatus | null = null;
  let snapshot: DebugSnapshot | null = null;
  let message: string | null = null;
  let failed = false;
  let refreshInFlight = false;
  let paletteRowsOpen = true;
  let backgroundWindowsOpen = true;
  let showScoreBreakdown = false;

  onMount(() => {
    refreshDebugSnapshot();
    const interval = window.setInterval(refreshDebugSnapshot, DEBUG_SNAPSHOT_REFRESH_MS);

    return () => window.clearInterval(interval);
  });

  function refreshDebugSnapshot() {
    if (refreshInFlight) {
      return;
    }

    refreshInFlight = true;
    Promise.all([paletteApi.getDebugOverlayStatus(), paletteApi.getDebugSnapshot()])
      .then(([nextStatus, nextSnapshot]) => {
        status = nextStatus;
        snapshot = nextSnapshot;
        message = null;
        failed = false;
      })
      .catch((error: unknown) => {
        message = errorMessage(error);
        failed = true;
      })
      .finally(() => {
        refreshInFlight = false;
      });
  }

  function windowLabel(window: { process_name: string | null; hwnd: number | null } | null) {
    if (!window) {
      return "None";
    }
    return `${window.process_name ?? "Unknown process"} (${window.hwnd ?? "no hwnd"})`;
  }

  function scoreBreakdownComponentText(row: DebugCommandRow) {
    const breakdown = row.score_breakdown;
    if (!breakdown) {
      return "";
    }

    return `label ${breakdown.label_score ?? 0} + tag ${breakdown.tag_contribution} + initials ${breakdown.word_initial_bonus} = raw ${breakdown.raw_score}`;
  }

  function scoreBreakdownEquationText(row: DebugCommandRow) {
    const breakdown = row.score_breakdown;
    if (!breakdown) {
      return "";
    }

    return `${breakdown.raw_score} x ${breakdown.focus_multiplier_percent}% focus x ${breakdown.priority_multiplier_percent}% priority x ${breakdown.favorite_multiplier_percent}% favorite + ${breakdown.priority_bonus} + ${breakdown.favorite_bonus} = ${breakdown.adjusted_score}`;
  }

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }
</script>

<main class="min-h-screen bg-zinc-950 p-4 text-zinc-100">
  <header>
    <div>
      <h1 class="text-lg font-semibold">Debug Overlay</h1>
      {#if status}
        <p class="mt-1 text-sm text-zinc-400">{formatDebugOverlayStatus(status)}</p>
      {/if}
    </div>
  </header>

  {#if message}
    <p
      class={[
        "mt-4 rounded border px-3 py-2 text-sm",
        failed
          ? "border-red-800 bg-red-950 text-red-200"
          : "border-emerald-800 bg-emerald-950 text-emerald-200",
      ].join(" ")}
    >
      {message}
    </p>
  {/if}

  {#if snapshot}
    <div class="mt-4 grid gap-4">
      <section class="rounded border border-zinc-800 bg-zinc-900 p-3">
        <h2 class="font-medium">Foreground</h2>
        <p class="mt-2 text-sm text-zinc-300">{windowLabel(snapshot.foreground_window)}</p>
        <p class="mt-1 text-sm text-zinc-400">
          Ignored: {snapshot.ignored_process_name ?? "No"}
        </p>
      </section>

      <section class="rounded border border-zinc-800 bg-zinc-900 p-3">
        <h2 class="font-medium">Interaction</h2>
        <p class="mt-2 text-sm text-zinc-300">
          Text input: {snapshot.text_input_active ? "active" : "inactive"}
        </p>
        <p class="mt-1 text-sm text-zinc-400">
          Tags: {snapshot.active_tags.length > 0 ? snapshot.active_tags.join(", ") : "None"}
        </p>
      </section>

      <section class="rounded border border-zinc-800 bg-zinc-900 p-3">
        <h2 class="font-medium">Command Candidates</h2>
        <div class="mt-2 grid grid-cols-2 gap-2 text-sm text-zinc-300">
          <span>Total: {snapshot.command_summary.total}</span>
          <span>Focused: {snapshot.command_summary.focused}</span>
          <span>Background: {snapshot.command_summary.background}</span>
          <span>Global: {snapshot.command_summary.global}</span>
          <span>Favorites: {snapshot.command_summary.favorites}</span>
          <span>High: {snapshot.command_summary.high_priority}</span>
          <span>Medium: {snapshot.command_summary.medium_priority}</span>
          <span>Low: {snapshot.command_summary.low_priority}</span>
          <span>Suppressed: {snapshot.command_summary.suppressed_priority}</span>
        </div>
      </section>

      <section class="rounded border border-zinc-800 bg-zinc-900 p-3">
        <div class="flex items-center justify-between gap-3">
          <button
            type="button"
            class="text-left font-medium text-zinc-100"
            aria-expanded={paletteRowsOpen}
            onclick={() => (paletteRowsOpen = !paletteRowsOpen)}
          >
            Palette Filter
          </button>
          <label class="flex items-center gap-2 text-xs text-zinc-400">
            <input
              type="checkbox"
              class="h-3 w-3 rounded border-zinc-700 bg-zinc-950"
              bind:checked={showScoreBreakdown}
            />
            <span>Score breakdown</span>
          </label>
        </div>
        <p class="mt-2 text-sm text-zinc-300">
          Query: {snapshot.palette_state.query || "(empty)"}
        </p>
        <p class="mt-1 text-sm text-zinc-400">
          Filtered rows: {snapshot.palette_state.filtered_count}
        </p>
        {#if paletteRowsOpen}
          {#if snapshot.palette_state.top_rows.length === 0}
            <p class="mt-3 text-sm text-zinc-500">No palette rows recorded yet.</p>
          {:else}
            <div class="mt-3 grid gap-2">
              {#each snapshot.palette_state.top_rows as row}
                <article class="rounded border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm">
                  <div class="font-medium">{row.label}</div>
                  <div class="mt-1 text-xs text-zinc-400">
                    {row.focus_state} - {row.priority} - score {row.score}
                  </div>
                  {#if showScoreBreakdown && row.score_breakdown}
                    <div class="mt-2 grid gap-1 text-xs text-zinc-500">
                      <div>{scoreBreakdownComponentText(row)}</div>
                      <div>{scoreBreakdownEquationText(row)}</div>
                      {#if row.score_breakdown.suppressed_bucket}
                        <div>suppressed bucket: bottom</div>
                      {/if}
                    </div>
                  {/if}
                </article>
              {/each}
            </div>
          {/if}
        {/if}
      </section>

      <section class="rounded border border-zinc-800 bg-zinc-900 p-3">
        <button
          type="button"
          class="text-left font-medium text-zinc-100"
          aria-expanded={backgroundWindowsOpen}
          onclick={() => (backgroundWindowsOpen = !backgroundWindowsOpen)}
        >
          Background Windows
        </button>
        <p class="mt-2 text-sm text-zinc-400">
          Showing {snapshot.background_windows.length} of {snapshot.background_total}
        </p>
        {#if backgroundWindowsOpen}
          {#if snapshot.background_windows.length === 0}
            <p class="mt-3 text-sm text-zinc-500">No background windows found.</p>
          {:else}
            <div class="mt-3 grid gap-2">
              {#each snapshot.background_windows as window}
                <div class="rounded border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm">
                  {windowLabel(window)}
                </div>
              {/each}
            </div>
          {/if}
        {/if}
      </section>
    </div>
  {:else}
    <p class="mt-4 text-sm text-zinc-400">Loading debug snapshot...</p>
  {/if}
</main>
