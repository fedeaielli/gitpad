<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import DOMPurify from 'dompurify'
  import type { Tab } from '../stores/tabs.svelte'
  import { errorMessage } from '../stores/git.svelte'
  import { logLine } from '../stores/toasts.svelte'
  import { imageMime } from '../utils/file-kind'
  import { renderScale } from '../utils/pdf-render'
  import { odfToHtml } from '../utils/odf'
  import { debounce } from '../utils/debounce'

  let { tab }: { tab: Tab } = $props()

  let bytes = $state<ArrayBuffer | null>(null)
  let loading = $state(true)
  let docLoading = $state(false)
  let error = $state<string | null>(null)
  let imageUrl = $state('')
  let docHtml = $state('')
  let pdfHost = $state<HTMLDivElement>()

  $effect(() => {
    const path = tab.path
    tab.reloadKey
    let cancelled = false
    bytes = null
    error = null
    loading = true
    void (async () => {
      try {
        const data = await invoke<ArrayBuffer>('fs_read_binary', { path })
        if (!cancelled) bytes = data
      } catch (e) {
        if (!cancelled) error = errorMessage(e)
      } finally {
        if (!cancelled) loading = false
      }
    })()
    return () => {
      cancelled = true
    }
  })

  // Images: a Blob URL keeps the bytes in memory once and lets <img> decode them.
  $effect(() => {
    const data = bytes
    if (!data || tab.view !== 'image') return
    const url = URL.createObjectURL(new Blob([data], { type: imageMime(tab.path) }))
    imageUrl = url
    return () => URL.revokeObjectURL(url)
  })

  // DOCX and ODT: convert to HTML once per read, then sanitize.
  $effect(() => {
    const data = bytes
    const view = tab.view
    if (!data || (view !== 'docx' && view !== 'odt')) return
    let cancelled = false
    docLoading = true
    docHtml = ''
    void (async () => {
      try {
        if (view === 'docx') {
          const mammoth = await import('mammoth')
          const result = await mammoth.convertToHtml(
            { arrayBuffer: data },
            { convertImage: mammoth.images.imgElement(() => Promise.resolve({ src: '' })) },
          )
          if (!cancelled) docHtml = DOMPurify.sanitize(result.value, { FORBID_TAGS: ['img'] })
        } else {
          const { strFromU8, unzipSync } = await import('fflate')
          const files = unzipSync(new Uint8Array(data))
          const content = files['content.xml']
          if (!content) throw new Error('content.xml missing from the ODT file')
          const styles = files['styles.xml']
          const html = odfToHtml(strFromU8(content), styles ? strFromU8(styles) : undefined)
          if (!cancelled) docHtml = DOMPurify.sanitize(html)
        }
      } catch (e) {
        if (!cancelled) error = errorMessage(e)
      } finally {
        if (!cancelled) docLoading = false
      }
    })()
    return () => {
      cancelled = true
    }
  })

  // PDF: one placeholder per page, each canvas rendered when it scrolls near.
  $effect(() => {
    const data = bytes
    const host = pdfHost
    if (!data || !host || tab.view !== 'pdf') return
    let cancelled = false
    let doc: import('pdfjs-dist').PDFDocumentProxy | undefined
    let loadingTask: import('pdfjs-dist').PDFDocumentLoadingTask | undefined
    let observer: IntersectionObserver | undefined
    let resize: ResizeObserver | undefined
    let rerender: ReturnType<typeof debounce> | undefined
    const rendered = new Set<number>()
    const wraps: HTMLDivElement[] = []

    void (async () => {
      try {
        const pdfjs = await import('pdfjs-dist')
        const worker = await import('pdfjs-dist/build/pdf.worker.min.mjs?url')
        pdfjs.GlobalWorkerOptions.workerSrc = worker.default
        loadingTask = pdfjs.getDocument({ data: new Uint8Array(data).slice() })
        doc = await loadingTask.promise
        if (cancelled) {
          void loadingTask.destroy()
          return
        }
        host.replaceChildren()
        for (let index = 1; index <= doc.numPages; index++) {
          const page = await doc.getPage(index)
          const size = page.getViewport({ scale: 1 })
          const wrap = document.createElement('div')
          wrap.className = 'pdf-page'
          wrap.dataset.page = String(index)
          wrap.style.aspectRatio = `${size.width} / ${size.height}`
          host.appendChild(wrap)
          wraps.push(wrap)
        }

        const render = async (index: number) => {
          if (!doc || rendered.has(index)) return
          rendered.add(index)
          const wrap = wraps[index - 1]
          const page = await doc.getPage(index)
          const base = page.getViewport({ scale: 1 })
          const viewport = page.getViewport({
            scale: renderScale(host.clientWidth, base.width, window.devicePixelRatio),
          })
          const canvas = document.createElement('canvas')
          canvas.width = Math.floor(viewport.width)
          canvas.height = Math.floor(viewport.height)
          wrap.replaceChildren(canvas)
          try {
            await page.render({ canvas, viewport }).promise
          } catch (e) {
            // A cancelled render after a tab switch is not an error, and one bad
            // page must not replace the whole document with an error message.
            if (!cancelled) logLine('warn', `pdf page ${index}: ${errorMessage(e)}`, 'editor')
          }
        }

        // Keep only the pages near the viewport in memory: a canvas is tens of
        // megabytes at high DPI, and a long document would exhaust the webview.
        const release = (index: number) => {
          if (!rendered.delete(index)) return
          wraps[index - 1].replaceChildren()
        }

        observer = new IntersectionObserver(
          (entries) => {
            for (const entry of entries) {
              const index = Number((entry.target as HTMLElement).dataset.page)
              if (entry.isIntersecting) void render(index)
              else release(index)
            }
          },
          { root: host, rootMargin: '300px' },
        )
        for (const wrap of wraps) observer.observe(wrap)

        // A width change invalidates the bitmap scale: redraw what is visible
        // once the drag settles. A height-only change keeps the same bitmaps.
        let lastWidth = host.clientWidth
        rerender = debounce(() => {
          const width = host.clientWidth
          if (width === lastWidth) return
          lastWidth = width
          for (const index of [...rendered]) {
            rendered.delete(index)
            void render(index)
          }
        }, 150)
        resize = new ResizeObserver(() => rerender?.run())
        resize.observe(host)
      } catch (e) {
        if (!cancelled) error = errorMessage(e)
      }
    })()

    return () => {
      cancelled = true
      observer?.disconnect()
      rerender?.cancel()
      resize?.disconnect()
      void loadingTask?.destroy()
      host.replaceChildren()
    }
  })
</script>

<div class="viewer">
  {#if loading}
    <p class="status">Loading…</p>
  {:else if error}
    <p class="status error" role="alert">{error}</p>
  {:else if tab.view === 'image'}
    <div class="image"><img src={imageUrl} alt={tab.path} /></div>
  {:else if tab.view === 'pdf'}
    <div class="pdf" bind:this={pdfHost}></div>
  {:else if docLoading}
    <p class="status">Loading…</p>
  {:else}
    <article class="doc">{@html docHtml}</article>
  {/if}
</div>

<style>
  .viewer {
    flex: 1;
    min-width: 0;
    overflow: auto;
    background: var(--surface-0);
  }
  .status {
    margin: 24px;
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .error {
    color: var(--err);
    overflow-wrap: anywhere;
  }
  .image {
    display: flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    height: 100%;
    padding: 16px;
  }
  .image img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  .pdf {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 16px;
  }
  .pdf :global(.pdf-page) {
    width: 100%;
    max-width: 1000px;
    background: var(--surface-1);
    border-radius: var(--r-sm);
    box-shadow: var(--shadow-1);
  }
  .pdf :global(canvas) {
    display: block;
    width: 100%;
    height: auto;
    border-radius: var(--r-sm);
  }
  .doc {
    box-sizing: border-box;
    max-width: 760px;
    margin: 0 auto;
    padding: 24px;
    color: var(--text);
    line-height: 1.6;
  }
  .doc :global(h1),
  .doc :global(h2),
  .doc :global(h3),
  .doc :global(h4),
  .doc :global(h5),
  .doc :global(h6) {
    margin: 1.2em 0 0.5em;
    line-height: 1.25;
  }
  .doc :global(p) {
    margin: 0.6em 0;
  }
  .doc :global(ul),
  .doc :global(ol) {
    margin: 0.6em 0;
    padding-left: 1.6em;
  }
  .doc :global(table) {
    border-collapse: collapse;
    margin: 0.8em 0;
  }
  .doc :global(th),
  .doc :global(td) {
    padding: 4px 8px;
    border: 1px solid var(--border);
    text-align: left;
    vertical-align: top;
  }
  .doc :global(a) {
    color: var(--accent-fg);
  }
</style>
