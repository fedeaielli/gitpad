import { invoke } from '@tauri-apps/api/core'

export interface UpdateInfo {
  current: string
  latest: string
  url: string
  newer: boolean
}

/// The latest published release; rejects when GitHub cannot be reached.
export function checkUpdate() {
  return invoke<UpdateInfo>('update_check')
}
