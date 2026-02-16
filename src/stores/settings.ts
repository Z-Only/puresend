/**
 * 设置状态管理
 */
import { defineStore } from 'pinia'
import {
    type ThemeMode,
    type LanguageMode,
    type SettingsState,
    DEFAULT_SETTINGS,
    SETTINGS_VERSION,
    SETTINGS_STORAGE_KEY,
} from '@/types/settings'

export const useSettingsStore = defineStore('settings', {
    state: (): SettingsState => ({
        ...DEFAULT_SETTINGS,
        version: SETTINGS_VERSION,
    }),

    getters: {
        /**
         * 获取实际主题（解析 system 模式）
         */
        actualTheme(): 'light' | 'dark' {
            if (this.theme === 'system') {
                try {
                    if (typeof window.matchMedia !== 'function') {
                        console.warn(
                            '[Settings] matchMedia API 不可用，使用默认浅色主题'
                        )
                        return 'light'
                    }
                    return window.matchMedia('(prefers-color-scheme: dark)')
                        .matches
                        ? 'dark'
                        : 'light'
                } catch (error) {
                    console.error('[Settings] 系统主题检测失败:', error)
                    return 'light'
                }
            }
            return this.theme
        },

        /**
         * 获取实际语言（解析 system 模式）
         */
        actualLanguage(): string {
            if (this.language === 'system') {
                try {
                    const systemLang =
                        navigator.language ||
                        (navigator.languages && navigator.languages.length > 0
                            ? navigator.languages[0]
                            : null) ||
                        'en-US'

                    if (!systemLang || typeof systemLang !== 'string') {
                        console.warn(
                            '[Settings] 系统语言检测返回无效值，使用英文'
                        )
                        return 'en-US'
                    }

                    const langMap: Record<string, string> = {
                        zh: 'zh-CN',
                        'zh-CN': 'zh-CN',
                        'zh-Hans': 'zh-CN',
                        'zh-Hans-CN': 'zh-CN',
                        'zh-TW': 'zh-CN',
                        'zh-HK': 'zh-CN',
                        en: 'en-US',
                        'en-US': 'en-US',
                        'en-GB': 'en-US',
                    }

                    if (langMap[systemLang]) {
                        return langMap[systemLang]
                    }

                    const prefix = systemLang.split('-')[0]
                    if (langMap[prefix]) {
                        return langMap[prefix]
                    }

                    return 'en-US'
                } catch (error) {
                    console.error('[Settings] 系统语言检测失败:', error)
                    return 'en-US'
                }
            }
            return this.language
        },
    },

    actions: {
        /**
         * 设置主题
         */
        async setTheme(theme: ThemeMode): Promise<boolean> {
            this.theme = theme
            return this.saveSettings()
        },

        /**
         * 设置语言
         */
        async setLanguage(language: LanguageMode): Promise<boolean> {
            this.language = language
            return this.saveSettings()
        },

        /**
         * 保存设置到持久化存储
         */
        async saveSettings(): Promise<boolean> {
            try {
                const settingsData: SettingsState = {
                    theme: this.theme,
                    language: this.language,
                    version: this.version,
                }

                if (await this.isTauriStoreAvailable()) {
                    await this.saveToTauriStore(settingsData)
                } else {
                    this.saveToLocalStorage(settingsData)
                }
                return true
            } catch (error) {
                console.error('[Settings] 保存设置失败:', error)
                return false
            }
        },

        /**
         * 从持久化存储加载设置
         */
        async loadSettings(): Promise<void> {
            try {
                let settings: SettingsState | null = null

                if (await this.isTauriStoreAvailable()) {
                    settings = await this.loadFromTauriStore()
                }

                if (!settings) {
                    settings = this.loadFromLocalStorage()
                }

                if (settings) {
                    if (settings.version !== SETTINGS_VERSION) {
                        settings = this.migrateSettings(settings)
                    }
                    this.theme = settings.theme
                    this.language = settings.language
                }
            } catch (error) {
                console.error('[Settings] 加载设置失败，使用默认值:', error)
            }
        },

        /**
         * 检查 Tauri Store 是否可用
         */
        async isTauriStoreAvailable(): Promise<boolean> {
            try {
                return typeof window !== 'undefined' && '__TAURI__' in window
            } catch {
                return false
            }
        },

        /**
         * 保存到 localStorage（降级方案）
         */
        saveToLocalStorage(data: SettingsState): void {
            localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(data))
        },

        /**
         * 从 localStorage 加载（降级方案）
         */
        loadFromLocalStorage(): SettingsState | null {
            const data = localStorage.getItem(SETTINGS_STORAGE_KEY)
            if (!data) return null
            try {
                return JSON.parse(data) as SettingsState
            } catch {
                console.warn('[Settings] localStorage 数据格式无效')
                return null
            }
        },

        /**
         * 保存到 Tauri Store
         */
        async saveToTauriStore(data: SettingsState): Promise<void> {
            try {
                const { Store } = await import('@tauri-apps/plugin-store')
                const store = await Store.load('settings.json')
                await store.set('settings', data)
                await store.save()
            } catch (error) {
                console.error('[Settings] Tauri Store 保存失败:', error)
                throw error
            }
        },

        /**
         * 从 Tauri Store 加载
         */
        async loadFromTauriStore(): Promise<SettingsState | null> {
            try {
                const { Store } = await import('@tauri-apps/plugin-store')
                const store = await Store.load('settings.json')
                const settings = await store.get<SettingsState>('settings')
                return settings ?? null
            } catch (error) {
                console.warn('[Settings] Tauri Store 加载失败:', error)
                return null
            }
        },

        /**
         * 设置迁移
         */
        migrateSettings(oldSettings: SettingsState): SettingsState {
            console.log(
                `[Settings] 迁移设置从版本 ${oldSettings.version} 到 ${SETTINGS_VERSION}`
            )
            return {
                ...oldSettings,
                version: SETTINGS_VERSION,
            }
        },

        /**
         * 监听系统主题变化
         */
        watchSystemTheme(
            callback: (theme: 'light' | 'dark') => void
        ): (() => void) | null {
            try {
                if (typeof window.matchMedia !== 'function') {
                    return null
                }

                const mediaQuery = window.matchMedia(
                    '(prefers-color-scheme: dark)'
                )

                const handler = (e: MediaQueryListEvent) => {
                    if (this.theme === 'system') {
                        callback(e.matches ? 'dark' : 'light')
                    }
                }

                mediaQuery.addEventListener('change', handler)

                return () => {
                    mediaQuery.removeEventListener('change', handler)
                }
            } catch (error) {
                console.error('[Settings] 系统主题监听初始化失败:', error)
                return null
            }
        },
    },
})
