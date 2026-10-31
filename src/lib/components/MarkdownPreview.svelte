<script lang="ts">
  import { marked } from 'marked'
  import DOMPurify from 'dompurify'
  import hljs from 'highlight.js/lib/common'
  import markdownDark from 'github-markdown-css/github-markdown-dark.css?url'
  import markdownLight from 'github-markdown-css/github-markdown-light.css?url'
  import codeDark from 'highlight.js/styles/github-dark.css?url'
  import codeLight from 'highlight.js/styles/github.css?url'
  import { registerThemeStyles } from '../utils/themes'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import { tabs } from '../stores/tabs.svelte'
  import { getEditor } from '../editor/bridge'

  registerThemeStyles({
    'gitpad-dark': { markdown: markdownDark, code: codeDark },
    'vs-dark': { markdown: markdownDark, code: codeDark },
    vs: { markdown: markdownLight, code: codeLight },
    'hc-black': { markdown: markdownDark, code: codeDark },
  })

  const renderer = new marked.Renderer()
  renderer.code = ({ text, lang }) => {
    const language = lang && hljs.getLanguage(lang) ? lang : 'plaintext'
    const value = hljs.highlight(text, { language }).value
    return `<pre><code class="hljs language-${language}">${value}</code></pre>`
  }
  marked.use({ renderer })

  let el = $state<HTMLDivElement | null>(null)
  let applyingToPreview = false
  let applyingToEditor = false

  const raw = $derived.by(() => {
    const tab = tabs.tabs.find((t) => t.id === tabs.activeTabId)
    return tab ? { id: tab.id, content: tab.content } : null
  })
  let html = $state('')
  let renderedTab = ''

  // Render immediately on tab switch, debounce while typing.
  $effect(() => {
    const source = raw
    if (!source) {
      html = ''
      return
    }
    const render = () => (html = DOMPurify.sanitize(marked.parse(source.content) as string))
    if (renderedTab !== source.id) {
      renderedTab = source.id
      render()
      return
    }
    const timer = setTimeout(render, 150)
    return () => clearTimeout(timer)
  })

  function syncFromEditor() {
    const node = el
    const editor = getEditor()
    if (!node || !editor) return
    const maxEditor = editor.getScrollHeight() - editor.getLayoutInfo().height
    const ratio = maxEditor > 0 ? editor.getScrollTop() / maxEditor : 0
    applyingToPreview = true
    node.scrollTop = ratio * Math.max(0, node.scrollHeight - node.clientHeight)
    requestAnimationFrame(() => (applyingToPreview = false))
  }

  function onScroll() {
    const node = el
    const editor = getEditor()
    if (!node || !editor || applyingToPreview) return
    const maxPreview = node.scrollHeight - node.clientHeight
    const ratio = maxPreview > 0 ? node.scrollTop / maxPreview : 0
    applyingToEditor = true
    editor.setScrollTop(ratio * Math.max(0, editor.getScrollHeight() - editor.getLayoutInfo().height))
    requestAnimationFrame(() => (applyingToEditor = false))
  }

  // Editor drives the preview; the flag stops the resulting scroll event from looping back.
  $effect(() => {
    const editor = getEditor()
    if (!editor) return
    const sub = editor.onDidScrollChange(() => {
      if (applyingToEditor) return
      syncFromEditor()
    })
    syncFromEditor()
    return () => sub.dispose()
  })

  function onClick(e: MouseEvent) {
    const anchor = (e.target as HTMLElement).closest('a')
    const href = anchor?.getAttribute('href')
    if (!href) return
    e.preventDefault()
    if (/^https?:\/\//.test(href)) void openUrl(href)
  }

  // Attached on the node so the container stays non-interactive for a11y.
  $effect(() => {
    const node = el
    if (!node) return
    node.addEventListener('click', onClick)
    return () => node.removeEventListener('click', onClick)
  })
</script>

<div class="preview" bind:this={el} onscroll={onScroll}>
  <article class="markdown-body">{@html html}</article>
</div>

<style>
  .preview {
    height: 100%;
    overflow: auto;
    background: var(--bg);
  }
  .markdown-body {
    min-width: 0;
    padding: 16px 24px;
    background: transparent;
    color: var(--text);
  }
</style>
