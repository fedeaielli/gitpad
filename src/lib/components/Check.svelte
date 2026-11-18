<script lang="ts">
  /// Sits inside a `<label class="field">`, so the row text toggles it and names
  /// it for screen readers; the native input stays focusable.
  let { checked = $bindable(false) }: { checked?: boolean } = $props()
</script>

<span class="check">
  <input type="checkbox" bind:checked />
  <svg viewBox="0 0 64 64" aria-hidden="true">
    <path
      class="path"
      pathLength="575.0541381835938"
      d="M 0 16 V 56 A 8 8 90 0 0 8 64 H 56 A 8 8 90 0 0 64 56 V 8 A 8 8 90 0 0 56 0 H 8 A 8 8 90 0 0 0 8 V 16 L 32 48 L 64 16 V 8 A 8 8 90 0 0 56 0 H 8 A 8 8 90 0 0 0 8 V 56 A 8 8 90 0 0 8 64 H 56 A 8 8 90 0 0 64 56 V 16"
    />
  </svg>
</span>

<style>
  .check {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: var(--sp-05);
  }
  input {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: 0;
    opacity: 0;
    pointer-events: none;
  }
  svg {
    width: 18px;
    height: 18px;
    overflow: visible;
  }
  .path {
    fill: none;
    stroke: var(--muted);
    stroke-width: 6;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-dasharray: 241 9999999;
    stroke-dashoffset: 0;
    transition:
      stroke-dasharray var(--t-base) var(--ease-out),
      stroke-dashoffset var(--t-base) var(--ease-out),
      stroke var(--t-fast);
  }
  input:checked + svg .path {
    stroke: var(--accent-fg);
    stroke-dasharray: 70.5096664428711 9999999;
    stroke-dashoffset: -262.2723388671875;
  }
  input:focus-visible + svg {
    outline: var(--focus-ring);
    outline-offset: 1px;
    border-radius: var(--r-sm);
  }
</style>
