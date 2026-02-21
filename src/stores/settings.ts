/**
 * 设置状态管理
 */
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
    type ThemeMode,
    type LanguageMode,
    type SettingsState,
    type HistoryPrivacySettings,
    type AutoCleanupSettings,
    DEFAULT_SETTINGS,
    DEFAULT_HISTORY_SETTINGS,
    SETTINGS_VERSION,
    SETTINGS_STORAGE_KEY,
} from '@/types/settings'
import type { AppLocale } from '@/i18n'
import {
    isTauriStoreAvailable,
    getDeviceName as getDeviceNameFromService,
    saveToTauriStore,
    loadFromTauriStore,
} from '@/services/settingsService'

export const useSettingsStore = defineStore('settings', () => {
    // ============ 状态 ============

    const deviceName = ref(DEFAULT_SETTINGS.deviceName)
    const theme = ref<ThemeMode>(DEFAULT_SETTINGS.theme)
    const language = ref<LanguageMode>(DEFAULT_SETTINGS.language)
    const history = ref(DEFAULT_HISTORY_SETTINGS)
    const version = ref(SETTINGS_VERSION)

    // ============ 计算属性 ============

    /**
     * 获取实际主题（解析 system 模式）
     */
    const actualTheme = computed((): 'light' | 'dark' => {
        if (theme.value === 'system') {
            try {
                if (typeof window.matchMedia !== 'function') {
                    console.warn(
                        '[Settings] matchMedia API 不可用，使用默认浅色主题'
                    )
                    return 'light'
                }
                return window.matchMedia('(prefers-color-scheme: dark)').matches
                    ? 'dark'
                    : 'light'
            } catch (error) {
                console.error('[Settings] 系统主题检测失败:', error)
                return 'light'
            }
        }
        return theme.value
    })

    /**
     * 获取实际语言（解析 system 模式）
     */
    const actualLanguage = computed((): AppLocale => {
        if (language.value === 'system') {
            try {
                const systemLang =
                    navigator.language ||
                    (navigator.languages && navigator.languages.length > 0
                        ? navigator.languages[0]
                        : null) ||
                    'en-US'

                if (!systemLang || typeof systemLang !== 'string') {
                    console.warn('[Settings] 系统语言检测返回无效值，使用英文')
                    return 'en-US'
                }

                const langMap: Record<string, AppLocale> = {
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
        return language.value as AppLocale
    })

    // ============ 方法 ============

    /**
     * 保存到 localStorage（降级方案）
     */
    function saveToLocalStorage(data: SettingsState): void {
        localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(data))
    }

    /**
     * 从 localStorage 加载（降级方案）
     */
    function loadFromLocalStorage(): SettingsState | null {
        const data = localStorage.getItem(SETTINGS_STORAGE_KEY)
        if (!data) return null
        try {
            return JSON.parse(data) as SettingsState
        } catch {
            console.warn('[Settings] localStorage 数据格式无效')
            return null
        }
    }

    /**
     * 保存到 Tauri Store（通过 service 层）
     */
    async function saveSettingsToTauriStore(
        data: SettingsState
    ): Promise<void> {
        const result = await saveToTauriStore('settings.json', 'settings', data)
        if (!result.success) {
            throw new Error(result.error || 'Tauri Store 保存失败')
        }
    }

    /**
     * 从 Tauri Store 加载（通过 service 层）
     */
    async function loadSettingsFromTauriStore(): Promise<SettingsState | null> {
        const result = await loadFromTauriStore<SettingsState>(
            'settings.json',
            'settings'
        )
        return result.data ?? null
    }

    /**
     * 设置迁移
     */
    function migrateSettings(oldSettings: SettingsState): SettingsState {
        console.log(
            `[Settings] 迁移设置从版本 ${oldSettings.version} 到 ${SETTINGS_VERSION}`
        )
        return {
            ...oldSettings,
            version: SETTINGS_VERSION,
            // 兼容旧版设置
            history: oldSettings.history || DEFAULT_HISTORY_SETTINGS,
        }
    }

    /**
     * 保存设置到持久化存储
     */
    async function saveSettings(): Promise<boolean> {
        try {
            const settingsData: SettingsState = {
                deviceName: deviceName.value,
                theme: theme.value,
                language: language.value,
                history: history.value,
                version: version.value,
            }

            if (await isTauriStoreAvailable()) {
                await saveSettingsToTauriStore(settingsData)
            } else {
                saveToLocalStorage(settingsData)
            }
            return true
        } catch (error) {
            console.error('[Settings] 保存设置失败:', error)
            return false
        }
    }

    /**
     * 从持久化存储加载设置
     */
    async function loadSettings(): Promise<void> {
        try {
            let settings: SettingsState | null = null

            if (await isTauriStoreAvailable()) {
                settings = await loadSettingsFromTauriStore()
            }

            if (!settings) {
                settings = loadFromLocalStorage()
            }

            if (settings) {
                if (settings.version !== SETTINGS_VERSION) {
                    settings = migrateSettings(settings)
                }
                deviceName.value = settings.deviceName || ''
                theme.value = settings.theme
                language.value = settings.language
                // 兼容旧版设置（没有 history 字段）
                history.value = settings.history || DEFAULT_HISTORY_SETTINGS

                // 如果没有设备名称，尝试获取本机设备名
                if (!deviceName.value) {
                    deviceName.value = await getSystemDeviceName()
                    await saveSettings()
                }
            }
        } catch (error) {
            console.error('[Settings] 加载设置失败，使用默认值:', error)
        }
    }

    /**
     * 获取本机设备名（通过 service 层）
     */
    async function getSystemDeviceName(): Promise<string> {
        return await getDeviceNameFromService()
    }

    /**
     * 设置设备名称
     */
    async function setDeviceName(name: string): Promise<boolean> {
        deviceName.value = name
        return saveSettings()
    }

    /**
     * 获取设备名称（如果未设置则获取本机设备名）
     */
    async function getDeviceName(): Promise<string> {
        if (deviceName.value) {
            return deviceName.value
        }
        // 尝试获取本机设备名
        const systemDeviceName = await getSystemDeviceName()
        if (systemDeviceName) {
            deviceName.value = systemDeviceName
            await saveSettings()
        }
        return deviceName.value
    }

    /**
     * 设置主题
     */
    async function setTheme(newTheme: ThemeMode): Promise<boolean> {
        theme.value = newTheme
        return saveSettings()
    }

    /**
     * 设置语言
     */
    async function setLanguage(newLanguage: LanguageMode): Promise<boolean> {
        language.value = newLanguage
        return saveSettings()
    }

    /**
     * 设置是否记录传输历史
     */
    async function setRecordHistory(
        recordHistoryValue: boolean
    ): Promise<boolean> {
        history.value = {
            ...history.value,
            recordHistory: recordHistoryValue,
        }
        return saveSettings()
    }

    /**
     * 设置历史记录隐私模式
     */
    async function setHistoryPrivacy(
        privacy: Partial<HistoryPrivacySettings>
    ): Promise<boolean> {
        history.value = {
            ...history.value,
            privacy: { ...history.value.privacy, ...privacy },
        }
        return saveSettings()
    }

    /**
     * 设置自动清理策略
     */
    async function setAutoCleanup(
        cleanup: Partial<AutoCleanupSettings>
    ): Promise<boolean> {
        history.value = {
            ...history.value,
            autoCleanup: { ...history.value.autoCleanup, ...cleanup },
        }
        return saveSettings()
    }

    /**
     * 监听系统主题变化
     */
    function watchSystemTheme(
        callback: (theme: 'light' | 'dark') => void
    ): (() => void) | null {
        try {
            if (typeof window.matchMedia !== 'function') {
                return null
            }

            const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')

            const handler = (e: MediaQueryListEvent) => {
                if (theme.value === 'system') {
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
    }

    return {
        // 状态
        deviceName,
        theme,
        language,
        history,
        version,

        // 计算属性
        actualTheme,
        actualLanguage,

        // 方法
        setDeviceName,
        getDeviceName,
        getSystemDeviceName,
        setTheme,
        setLanguage,
        setRecordHistory,
        setHistoryPrivacy,
        setAutoCleanup,
        saveSettings,
        loadSettings,
        watchSystemTheme,
    }
})
