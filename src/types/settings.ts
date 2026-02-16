/**
 * 设置相关类型定义
 */

/** 主题模式 */
export type ThemeMode = 'light' | 'dark' | 'system'

/** 语言模式 */
export type LanguageMode = 'zh-CN' | 'en-US' | 'system'

/** 应用设置 */
export interface AppSettings {
    /** 主题模式 */
    theme: ThemeMode
    /** 语言模式 */
    language: LanguageMode
}

/** 设置存储版本，用于迁移兼容 */
export const SETTINGS_VERSION = 1

/** 扩展的设置状态，包含版本信息 */
export interface SettingsState extends AppSettings {
    version: number
}

/** 默认设置 */
export const DEFAULT_SETTINGS: AppSettings = {
    theme: 'system',
    language: 'system',
}

/** 本地存储键名 */
export const SETTINGS_STORAGE_KEY = 'puresend-settings'
