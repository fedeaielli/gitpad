<script lang="ts">
  import { languageFor } from '../utils/langmap'

  let {
    name,
    dir = false,
    size = 14,
  }: { name: string; dir?: boolean; size?: number } = $props()

  const IMAGE_EXT = ['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'ico', 'bmp']
  // Languages drawn as a code glyph; the colour is what names them.
  const CODE = ['javascript', 'typescript', 'rust', 'go', 'java', 'c', 'cpp', 'shell', 'python']

  const ext = $derived(name.split('.').pop()?.toLowerCase() ?? '')
  const lang = $derived(languageFor(name))
  const image = $derived(IMAGE_EXT.includes(ext))

  const glyph = $derived(
    dir
      ? 'dir'
      : image
        ? 'image'
        : lang === 'markdown'
          ? 'markdown'
          : lang === 'json'
            ? 'json'
            : CODE.includes(lang)
              ? 'code'
              : 'text',
  )

  // Colours are mid-tones: readable on both the dark and the light surface.
  const COLORS: Record<string, string> = {
    dir: 'var(--accent-fg)',
    image: '#a074c4',
    markdown: '#519aba',
    json: '#b8b830',
    python: '#4b8bbe',
    typescript: '#3178c6',
    javascript: '#b8a02a',
    rust: '#c97a5e',
    go: '#00add8',
    java: '#b07219',
    shell: '#4eaa25',
    yaml: '#6d8086',
    toml: '#6d8086',
  }
  const color = $derived(COLORS[dir ? 'dir' : image ? 'image' : lang] ?? 'currentColor')
</script>

<svg
  class="fi"
  width={size}
  height={size}
  viewBox="0 0 16 16"
  aria-hidden="true"
  style:color
>
  {#if glyph === 'dir'}
    <path d="M1.5 4.3h4.4l1.4 1.6h7.2v7.6H1.5z" fill="currentColor" />
  {:else if glyph === 'code'}
    <path d="M6.1 5.2 2.9 8l3.2 2.8M9.9 5.2 13.1 8l-3.2 2.8" />
  {:else if glyph === 'json'}
    <path
      d="M6.2 4.2c-1.1 0-1.4.6-1.4 1.5v1c0 .8-.4 1.1-1.1 1.3.7.2 1.1.5 1.1 1.3v1c0 .9.3 1.5 1.4 1.5M9.8 4.2c1.1 0 1.4.6 1.4 1.5v1c0 .8.4 1.1 1.1 1.3-.7.2-1.1.5-1.1 1.3v1c0 .9-.3 1.5-1.4 1.5"
    />
  {:else}
    <path d="M4 1.8h5.4l2.8 2.8v9.6H4z" />
    <path d="M9.4 1.8v2.8h2.8" />
    {#if glyph === 'markdown'}
      <path d="M8 7.4v3.4M6.4 9.3 8 10.9l1.6-1.6" />
    {:else if glyph === 'image'}
      <path d="M4 9.6 6.6 7l2.2 2.2 1.4-1.4 1.8 1.8" />
      <circle cx="6" cy="5.6" r=".9" />
    {:else}
      <path d="M6 7h4M6 9h4M6 11h2.6" />
    {/if}
  {/if}
</svg>

<style>
  .fi {
    flex: 0 0 auto;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
