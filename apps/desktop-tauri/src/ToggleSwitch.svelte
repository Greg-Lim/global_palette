<script lang="ts">
  export let checked: boolean;
  export let ariaLabel: string;
  export let disabled = false;
  export let updating = false;
  export let onToggle: (checked: boolean) => void;

  function handleChange(event: Event) {
    if (disabled || updating) {
      return;
    }

    onToggle((event.currentTarget as HTMLInputElement).checked);
  }
</script>

<label
  class={[
    "toggle-switch",
    disabled || updating ? "toggle-switch-disabled" : "toggle-switch-enabled",
  ].join(" ")}
>
  <input
    aria-label={ariaLabel}
    checked={checked}
    class="sr-only"
    disabled={disabled || updating}
    onchange={handleChange}
    role="switch"
    type="checkbox"
  />
  <span
    aria-hidden="true"
    class={[
      "toggle-switch-track",
      checked ? "toggle-switch-track-on" : "toggle-switch-track-off",
    ].join(" ")}
  >
    <span
      class={[
        "toggle-switch-thumb",
        checked ? "toggle-switch-thumb-on" : "toggle-switch-thumb-off",
      ].join(" ")}
    ></span>
  </span>
  {#if updating}
    <span aria-hidden="true" class="toggle-switch-spinner"></span>
  {/if}
</label>
