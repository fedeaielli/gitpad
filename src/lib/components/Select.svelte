<script module lang="ts">
  // One id per instance: the options need names for aria-activedescendant.
  let seq = 0

  export type SelectOption<T extends string> = { value: T; label: string }
</script>

<script lang="ts" generics="T extends string">
  /// Drop-in for a native <select>: GTK draws the native popup in its own
  /// colours, so the list flashes white in every theme. This one is ours.
  import { stepIndex } from '../utils/select'
  import { portal } from '../utils/portal'

  let {
    value = $bindable(),
    options,
    id,
    ariaLabelledby,
    disabled = false,
    onchange,
  }: {
    value: T
    options: SelectOption<T>[]
    id?: string
    ariaLabelledby?: string
    disabled?: boolean
    onchange?: () => void
  } = $props()

  let open = $state(false)
  let active = $state(0)
  let listEl = $state<HTMLElement>()
  let trigger = $state<HTMLButtonElement>()
  let wrapper = $state<HTMLDivElement>()
  let layerEl = $state<HTMLDivElement>()
  /// Where the list sits, measured on open: the modal body scrolls and clips,
  /// so the popup is fixed to the viewport rather than to its field.
  let anchor = $state<{ top: number; left: number; width: number; max: number; up: boolean } | null>(
    null,
  )

  const uid = `gp-select-${++seq}`
  const current = $derived(options.findIndex((option) => option.value === value))

  function openList() {
    if (disabled) return
    active = Math.max(0, current)
    const box = trigger?.getBoundingClientRect()
    if (box) {
      // Flip up when the list would not fit below.
      const up = box.bottom + 260 > window.innerHeight
      anchor = {
        top: up ? box.top - 4 : box.bottom + 4,
        left: box.left,
        width: box.width,
        max: Math.max(120, up ? box.top - 12 : window.innerHeight - box.bottom - 12),
        up,
      }
    }
    open = true
  }

  function closeList() {
    open = false
  }

  function commit(index: number) {
    const next = options[index]
    open = false
    if (!next || next.value === value) return
    value = next.value
    onchange?.()
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && open) {
      event.preventDefault()
      // The host dialog also closes on Escape; this one is closer.
      event.stopPropagation()
      closeList()
      return
    }
    if (event.key === 'Tab') {
      closeList()
      return
    }
    if (!open && (event.key === 'Enter' || event.key === ' ')) {
      event.preventDefault()
      openList()
      return
    }
    if (open && (event.key === 'Enter' || event.key === ' ')) {
      event.preventDefault()
      commit(active)
      return
    }
    const next = stepIndex(open ? active : current, options.length, event.key)
    if (next === null) return
    event.preventDefault()
    if (!open) {
      openList()
      return
    }
    active = next
  }

  // Keep the highlighted entry inside the scrollable list.
  $effect(() => {
    if (!open) return
    active
    listEl?.querySelector('.option.active')?.scrollIntoView({ block: 'nearest' })
  })

  // A fixed list cannot follow the field it belongs to; drop it instead.
  $effect(() => {
    if (!open) return
    const drop = () => closeList()
    window.addEventListener('scroll', drop, true)
    window.addEventListener('resize', drop)
    return () => {
      window.removeEventListener('scroll', drop, true)
      window.removeEventListener('resize', drop)
    }
  })

  // Second net, for whatever stacks above the scrim (a tooltip, a toast): a
  // click there still dismisses and must not reach the element under it.
  $effect(() => {
    if (!open) return
    const outside = (event: MouseEvent) => {
      const target = event.target
      if (target instanceof Node && (wrapper?.contains(target) || layerEl?.contains(target))) return
      closeList()
      event.preventDefault()
      event.stopPropagation()
    }
    window.addEventListener('click', outside, true)
    return () => window.removeEventListener('click', outside, true)
  })

  // The list sits in the top layer. Inside the modal's scroller a fixed box is
  // clipped and mis-hit-tested, so clicks fell through to the row underneath;
  // a top-layer box has the viewport as its containing block and no ancestors.
  $effect(() => {
    const el = layerEl
    if (!el || typeof el.showPopover !== 'function') return
    // shortcut: without popover support the layer stays a plain fixed box.
    if (open && !el.matches(':popover-open')) el.showPopover()
    else if (!open && el.matches(':popover-open')) el.hidePopover()
  })
</script>

<div class="wrap" bind:this={wrapper}>
  <button
    {id}
    bind:this={trigger}
    class="trigger"
    type="button"
    role="combobox"
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={open ? `${uid}-list` : undefined}
    aria-activedescendant={open ? `${uid}-${active}` : undefined}
    aria-labelledby={ariaLabelledby}
    {disabled}
    onclick={() => (open ? closeList() : openList())}
    onkeydown={onKeydown}
  >
    <span class="value">{options[current]?.label ?? options[0]?.label ?? ''}</span>
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <path d="m4 6 4 4 4-4" />
    </svg>
  </button>

  <div class="layer" popover="manual" use:portal bind:this={layerEl}>
    {#if open}
      <button
        class="scrim"
        type="button"
        tabindex="-1"
        aria-label="Close list"
        onclick={closeList}
      ></button>
      <div
        class="popup"
        id="{uid}-list"
        role="listbox"
        bind:this={listEl}
        class:up={anchor?.up}
        style:top={anchor && !anchor.up ? `${anchor.top}px` : 'auto'}
        style:bottom={anchor?.up ? `${window.innerHeight - anchor.top}px` : 'auto'}
        style:left="{anchor?.left ?? 0}px"
        style:min-width="{anchor?.width ?? 0}px"
        style:max-height="{Math.min(240, anchor?.max ?? 240)}px"
      >
        {#each options as option, i (option.value)}
          <button
            id="{uid}-{i}"
            class="option"
            class:active={i === active}
            type="button"
            role="option"
            aria-selected={option.value === value}
            tabindex="-1"
            onmousemove={() => (active = i)}
            onclick={() => commit(i)}
          >
            {option.label}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .wrap {
    display: inline-block;
    width: 100%;
    /* A field can cap its control; see Settings. */
    max-width: var(--select-max, none);
  }
  .trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-2);
    width: 100%;
    box-sizing: border-box;
    height: 32px;
    padding: 0 var(--sp-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    background: var(--surface-0);
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .trigger:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .value {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .trigger svg {
    flex: 0 0 auto;
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linecap: round;
  }
  /* The top-layer box: viewport-sized, so the anchor's viewport numbers apply
     unchanged. It never paints; the scrim and the list do. */
  .layer {
    position: fixed;
    inset: 0;
    /* Above the settings overlay, below the toasts. The top layer wins over
       this whenever the webview supports popovers. */
    z-index: 90;
    /* The UA stylesheet gives a popover `width/height: fit-content`; the layer
       must span the viewport or the scrim would only cover the list. */
    width: auto;
    height: auto;
    margin: 0;
    padding: 0;
    overflow: visible;
    border: 0;
    background: transparent;
    pointer-events: none;
  }
  /* The dismissing click lands here, so nothing underneath can receive it. */
  .scrim {
    position: absolute;
    inset: 0;
    border: 0;
    background: transparent;
    cursor: default;
    pointer-events: auto;
  }
  .popup {
    position: absolute;
    pointer-events: auto;
    box-sizing: border-box;
    overflow-y: auto;
    padding: var(--sp-05);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-3);
    box-shadow: var(--shadow-1);
    font-size: var(--fs-sm);
    /* Grows out of the field it was clicked on, not from its centre. */
    transform-origin: top left;
    animation: gp-menu-in var(--t-fast) var(--ease-out);
  }
  .popup.up {
    transform-origin: bottom left;
  }
  .option {
    display: block;
    width: 100%;
    padding: var(--sp-1) var(--sp-2);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
  }
  .option.active {
    background: var(--accent);
    color: var(--on-accent);
  }
</style>
