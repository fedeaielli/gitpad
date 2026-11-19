import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { logLine, pushToast } from './lib/stores/toasts.svelte'

// Dev only: fire toasts from the console to check stacking and dismissal.
if (import.meta.env.DEV) Object.assign(window, { pushToast })

// With the devtools closed a UI crash leaves nothing behind: keep the line.
window.addEventListener('error', (event) => {
  logLine('error', `ui: ${event.message}`)
})
window.addEventListener('unhandledrejection', (event) => {
  logLine('error', `ui: ${String(event.reason)}`)
})

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
