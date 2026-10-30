import { invoke } from '@tauri-apps/api/core'

export interface Config {
  onboardingComplete: boolean
  theme: string
  previewMode: string
}

export const loadConfig = () => invoke<Config>('config_get')

export const saveConfig = (config: Config) => invoke<void>('config_save', { config })
