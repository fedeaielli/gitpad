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
      // Same ESM build our own import resolves to; without it the CJS require in
      // monaco-emacs picks monaco-editor's AMD bundle.
      { find: /^monaco-editor$/, replacement: monaco('editor.main.js') },
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
