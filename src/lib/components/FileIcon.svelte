<script lang="ts">
  import { getIconForFile, getIconForFolder } from 'vscode-icons-js'

  let {
    name,
    dir = false,
    size = 14,
  }: { name: string; dir?: boolean; size?: number } = $props()

  // The SVGs are inlined as data URLs: the whole tree then costs no requests,
  // and each icon stays a closed document, so the gradients keep their ids.
  const RAW = import.meta.glob('../icons/*.svg', {
    query: '?raw',
    import: 'default',
    eager: true,
  }) as Record<string, string>

  const ICONS: Record<string, string> = {}
  for (const [path, svg] of Object.entries(RAW)) {
    const icon = path.slice(path.lastIndexOf('/') + 1, -'.svg'.length)
    ICONS[icon] = `data:image/svg+xml,${encodeURIComponent(svg.replace(/\s+/g, ' '))}`
  }

  const FOLDER = ICONS['default_folder']
  const FILE = ICONS['default_file']

  // We ship one set, not the light-theme twin the mapping keeps asking for.
  const icon = $derived.by(() => {
    const picked = (dir ? getIconForFolder(name) : getIconForFile(name)) ?? ''
    const flat = picked.replace('file_type_light_', 'file_type_').replace(/\.svg$/, '')
    return ICONS[flat] ?? (dir ? FOLDER : FILE)
  })
</script>

<img class="fi" src={icon} width={size} height={size} alt="" aria-hidden="true" />

<style>
  .fi {
    flex: 0 0 auto;
    display: block;
  }
</style>
