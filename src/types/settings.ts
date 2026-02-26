/**
 * 设置相关类型定义
 */

/** 主题模式 */
export type ThemeMode = 'light' | 'dark' | 'system'

/** 语言模式 */
export type LanguageMode = 'zh-CN' | 'en-US' | 'system'

/** Tab 栏布局模式 */
export type TabLayout =
    | 'horizontal-top'
    | 'horizontal-bottom'
    | 'vertical-left'
    | 'vertical-right'

/** 字体大小模式 */
export type FontSizeMode = 'system' | 'preset' | 'custom'

/** 字体大小预设 */
export type FontSizePreset = 'small' | 'medium' | 'large' | 'xlarge'

/** 字体大小设置 */
export interface FontSizeSettings {
    /** 模式 */
    mode: FontSizeMode
    /** 预设大小（仅在 mode='preset' 时有效） */
    preset: FontSizePreset
    /** 自定义缩放比例（仅在 mode='custom' 时有效） */
    customScale: number
}

/** 设置存储版本，用于迁移兼容 */
export const SETTINGS_VERSION = 7

/** 清理策略 */
export type CleanupStrategy = 'byTime' | 'byCount' | 'disabled'

/** 历史记录隐私设置 */
export interface HistoryPrivacySettings {
    /** 是否启用隐私模式 */
    enabled: boolean
    /** 隐藏文件名 */
    hideFileName: boolean
    /** 隐藏对端设备名 */
    hidePeerName: boolean
}

/** 自动清理设置 */
export interface AutoCleanupSettings {
    /** 清理策略 */
    strategy: CleanupStrategy
    /** 按时间清理：保留天数（仅在 strategy='byTime' 时有效） */
    retentionDays?: number
    /** 按数量清理：保留条数（仅在 strategy='byCount' 时有效） */
    maxCount?: number
}

/** 历史记录设置 */
export interface HistorySettings {
    /** 是否记录传输历史 */
    recordHistory: boolean
    /** 隐私模式设置 */
    privacy: HistoryPrivacySettings
    /** 自动清理设置 */
    autoCleanup: AutoCleanupSettings
}

/** 端口范围 */
export interface PortRange {
    /** 最小端口（0 表示系统自动分配） */
    minPort: number
    /** 最大端口（0 表示系统自动分配） */
    maxPort: number
}

/** 端口范围配置 */
export interface PortRangeConfig {
    /** 文件接收服务器端口范围 */
    transfer: PortRange
    /** HTTP 上传服务器端口范围 */
    webUpload: PortRange
    /** HTTP 下载服务器端口范围 */
    share: PortRange
}

/** 开发者设置 */
export interface DeveloperSettings {
    /** DevTools 开关 */
    devToolsEnabled: boolean
    /** 端口范围配置 */
    portRange: PortRangeConfig
}

/** 应用设置 */
export interface AppSettings {
    /** 设备名称 */
    deviceName: string
    /** 主题模式 */
    theme: ThemeMode
    /** 语言模式 */
    language: LanguageMode
    /** 历史记录设置 */
    history: HistorySettings
    /** Tab 栏布局模式 */
    tabLayout: TabLayout
    /** 字体大小设置 */
    fontSize: FontSizeSettings
    /** 是否显示高级设置 */
    showAdvancedSettings: boolean
}

/** 扩展的设置状态，包含版本信息 */
export interface SettingsState extends AppSettings {
    version: number
    /** 接收设置 */
    receiveSettings?: ReceiveSettings
    /** 开发者设置 */
    developerSettings?: DeveloperSettings
}

/** 默认隐私设置 */
export const DEFAULT_PRIVACY_SETTINGS: HistoryPrivacySettings = {
    enabled: false,
    hideFileName: true,
    hidePeerName: false,
}

/** 默认自动清理设置 */
export const DEFAULT_AUTO_CLEANUP_SETTINGS: AutoCleanupSettings = {
    strategy: 'disabled',
}

/** 默认字体大小设置 */
export const DEFAULT_FONT_SIZE_SETTINGS: FontSizeSettings = {
    mode: 'system',
    preset: 'medium',
    customScale: 1.0,
}

/** 默认端口范围 */
export const DEFAULT_PORT_RANGE: PortRange = {
    minPort: 0,
    maxPort: 0,
}

/** 默认端口范围配置 */
export const DEFAULT_PORT_RANGE_CONFIG: PortRangeConfig = {
    transfer: { ...DEFAULT_PORT_RANGE },
    webUpload: { ...DEFAULT_PORT_RANGE },
    share: { ...DEFAULT_PORT_RANGE },
}

/** 默认开发者设置 */
export const DEFAULT_DEVELOPER_SETTINGS: DeveloperSettings = {
    devToolsEnabled: false,
    portRange: DEFAULT_PORT_RANGE_CONFIG,
}

/** 默认历史记录设置 */
export const DEFAULT_HISTORY_SETTINGS: HistorySettings = {
    recordHistory: true,
    privacy: DEFAULT_PRIVACY_SETTINGS,
    autoCleanup: DEFAULT_AUTO_CLEANUP_SETTINGS,
}

/** 默认设置 */
export const DEFAULT_SETTINGS: AppSettings = {
    deviceName: '',
    theme: 'system',
    language: 'system',
    history: DEFAULT_HISTORY_SETTINGS,
    tabLayout: 'horizontal-top',
    fontSize: DEFAULT_FONT_SIZE_SETTINGS,
    showAdvancedSettings: false,
}

/** 接收设置 */
export interface ReceiveSettings {
    /** 是否自动接收（原 autoSave） */
    autoReceive: boolean
    /** 是否覆盖同名文件 */
    fileOverwrite: boolean
    /** 请求过期时间（秒） */
    requestExpireTime: number
    /** 最大待处理请求数量 */
    maxPendingRequests: number
}

/** 默认接收设置 */
export const DEFAULT_RECEIVE_SETTINGS: ReceiveSettings = {
    autoReceive: false,
    fileOverwrite: false,
    requestExpireTime: 300,
    maxPendingRequests: 50,
}

/** 本地存储键名 */
export const SETTINGS_STORAGE_KEY = 'puresend-settings'
