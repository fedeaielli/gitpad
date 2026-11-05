import { invoke } from '@tauri-apps/api/core'
import { pushToast } from '../stores/toasts.svelte'

/// Opens a link in the browser. When no opener works the link is copied, so a
/// bare desktop cannot leave the user stuck on the login screen.
export async function openExternal(url: string) {
  try {
    await invoke('open_external', { url })
  } catch {
    try {
      await navigator.clipboard.writeText(url)
      pushToast("Couldn't open the browser — link copied to the clipboard.", 'warning', 'app')
    } catch {
      pushToast(`Couldn't open the browser: ${url}`, 'warning', 'app')
    }
  }
}
