import { invoke } from '@tauri-apps/api/core'

export interface Config {
  onboardingComplete: boolean
  theme: string
  previewMode: string
  fontSize: number
  fontFamily: string
  sidebarWidth: number
  sidebarOpen: boolean
  autoSync: boolean
  saveDebounceMs: number
  pushIdleMs: number
  keybindings: string
  extensions: Record<string, string>
  accent: string
  repoPath: string
  sortMode: string
  sortDesc: boolean
  foldersFirst: boolean
  showHidden: boolean
}

export const loadConfig = () => invoke<Config>('config_get')

export const saveConfig = (config: Config) => invoke<void>('config_save', { config })
