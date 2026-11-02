import { svelte } from '@sveltejs/vite-plugin-svelte'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'

// monaco-vim imports deep monaco paths that the package exports map blocks.
const monaco = (path: string) =>
  fileURLToPath(new URL(`./node_modules/monaco-editor/esm/vs/editor/${path}`, import.meta.url))

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: [
      // The full ESM entry, not editor.api.js: the API-only build registers no
      // contributions, so find, undo, clipboard and comment actions vanish.
      {
        find: /^monaco-editor$/,
        replacement: fileURLToPath(
          new URL('./node_modules/monaco-editor/esm/vs/index.js', import.meta.url),
        ),
      },
      {
        find: 'monaco-editor/esm/vs/editor/editor.api',
        replacement: monaco('editor.api.js'),
      },
      {
        find: 'monaco-editor/esm/vs/editor/common/commands/shiftCommand',
        replacement: monaco('common/commands/shiftCommand.js'),
      },
    ],
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
})
