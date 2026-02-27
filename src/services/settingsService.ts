/**
 * 设置服务 - Tauri 命令封装
 *
 * 提供设置相关的 Tauri API 调用封装，支持非 Tauri 环境降级
 */

import type { UnlistenFn } from '@tauri-apps/api/event'

// ============ 环境检测 ============

/**
 * 检查 Tauri 环境是否可用
 */
export async function isTauriEnvironmentAvailable(): Promise<boolean> {
    try {
        return (
            typeof window !== 'undefined' &&
            ('__TAURI__' in window || '__TAURI_INTERNALS__' in window)
        )
    } catch {
        return false
    }
}

// ============ 设备名称 ============

/**
 * 获取本机设备名称
 * @returns 设备名称，非 Tauri 环境返回默认值
 */
export async function getDeviceName(): Promise<string> {
    try {
        if (await isTauriEnvironmentAvailable()) {
            const { invoke } = await import('@tauri-apps/api/core')
            return await invoke<string>('get_device_name')
        }
    } catch (error) {
        console.warn('[SettingsService] 获取本机设备名失败:', error)
    }
    // 降级方案：使用浏览器信息或默认值
    return navigator.userAgent.includes('Mobile')
        ? 'Mobile Device'
        : 'Desktop Device'
}

// ============ Tauri Store 操作 ============

/**
 * Tauri Store 操作结果
 */
export interface TauriStoreResult<T> {
    success: boolean
    data?: T
    error?: string
}

/**
 * 检查 Tauri Store 是否可用
 */
export async function isTauriStoreAvailable(): Promise<boolean> {
    try {
        if (!(await isTauriEnvironmentAvailable())) {
            return false
        }
        // 尝试动态导入 plugin-store 来验证是否可用
        await import('@tauri-apps/plugin-store')
        return true
    } catch {
        return false
    }
}

/**
 * 保存数据到 Tauri Store
 * @param storeFile Store 文件名
 * @param key 数据键
 * @param value 数据值
 */
export async function saveToTauriStore<T>(
    storeFile: string,
    key: string,
    value: T
): Promise<TauriStoreResult<void>> {
    try {
        const { Store } = await import('@tauri-apps/plugin-store')
        const store = await Store.load(storeFile)
        await store.set(key, value)
        await store.save()
        return { success: true }
    } catch (error) {
        const errorMessage =
            error instanceof Error ? error.message : String(error)
        console.error('[SettingsService] Tauri Store 保存失败:', error)
        return { success: false, error: errorMessage }
    }
}

/**
 * 从 Tauri Store 加载数据
 * @param storeFile Store 文件名
 * @param key 数据键
 */
export async function loadFromTauriStore<T>(
    storeFile: string,
    key: string
): Promise<TauriStoreResult<T | null>> {
    try {
        const { Store } = await import('@tauri-apps/plugin-store')
        const store = await Store.load(storeFile)
        const data = await store.get<T>(key)
        return { success: true, data: data ?? null }
    } catch (error) {
        const errorMessage =
            error instanceof Error ? error.message : String(error)
        console.warn('[SettingsService] Tauri Store 加载失败:', error)
        return { success: false, error: errorMessage, data: null }
    }
}

// ============ 事件监听 ============

/**
 * 监听设置变更事件
 * @param callback 回调函数
 * @returns 取消监听函数
 */
export async function onSettingsChange(
    callback: (key: string, value: unknown) => void
): Promise<UnlistenFn | null> {
    try {
        if (!(await isTauriEnvironmentAvailable())) {
            return null
        }
        const { listen } = await import('@tauri-apps/api/event')
        return listen<{ key: string; value: unknown }>(
            'settings-change',
            (event) => {
                callback(event.payload.key, event.payload.value)
            }
        )
    } catch (error) {
        console.warn('[SettingsService] 事件监听注册失败:', error)
        return null
    }
}

// ============ 接收设置 ============

/** 接收设置 */
export interface ReceiveSettings {
    autoReceive: boolean
    fileOverwrite: boolean
}

/**
 * 获取接收设置
 */
export async function getReceiveSettings(): Promise<ReceiveSettings> {
    try {
        if (await isTauriEnvironmentAvailable()) {
            const { invoke } = await import('@tauri-apps/api/core')
            return await invoke<ReceiveSettings>('get_receive_settings')
        }
    } catch (error) {
        console.warn('[SettingsService] 获取接收设置失败:', error)
    }
    return { autoReceive: false, fileOverwrite: false }
}

/**
 * 设置自动接收
 */
export async function setAutoReceive(enabled: boolean): Promise<void> {
    try {
        if (await isTauriEnvironmentAvailable()) {
            const { invoke } = await import('@tauri-apps/api/core')
            await invoke('set_auto_receive', { enabled })
        }
    } catch (error) {
        console.warn('[SettingsService] 设置自动接收失败:', error)
    }
}

/**
 * 设置文件覆盖
 */
export async function setFileOverwrite(enabled: boolean): Promise<void> {
    try {
        if (await isTauriEnvironmentAvailable()) {
            const { invoke } = await import('@tauri-apps/api/core')
            await invoke('set_file_overwrite', { enabled })
        }
    } catch (error) {
        console.warn('[SettingsService] 设置文件覆盖失败:', error)
    }
}

// ============ 传输加密设置 ============

/**
 * 设置传输加密开关
 */
export async function setEncryptionEnabled(enabled: boolean): Promise<void> {
    try {
        if (await isTauriEnvironmentAvailable()) {
            const { invoke } = await import('@tauri-apps/api/core')
            await invoke('set_encryption_enabled', { enabled })
        }
    } catch (error) {
        console.warn('[SettingsService] 设置传输加密失败:', error)
    }
}

/**
 * 获取传输加密状态
 */
export async function getEncryptionEnabled(): Promise<boolean> {
    try {
        if (await isTauriEnvironmentAvailable()) {
            const { invoke } = await import('@tauri-apps/api/core')
            return await invoke<boolean>('get_encryption_enabled')
        }
    } catch (error) {
        console.warn('[SettingsService] 获取传输加密状态失败:', error)
    }
    return true
}

// ============ 动态压缩设置 ============

/**
 * 设置压缩开关
 */
export async function setCompressionEnabled(enabled: boolean): Promise<void> {
    try {
        if (await isTauriEnvironmentAvailable()) {
            const { invoke } = await import('@tauri-apps/api/core')
            await invoke('set_compression_enabled', { enabled })
        }
    } catch (error) {
        console.warn('[SettingsService] 设置压缩开关失败:', error)
    }
}

/**
 * 设置压缩模式
 */
export async function setCompressionMode(mode: string): Promise<void> {
    try {
        if (await isTauriEnvironmentAvailable()) {
            const { invoke } = await import('@tauri-apps/api/core')
            await invoke('set_compression_mode', { mode })
        }
    } catch (error) {
        console.warn('[SettingsService] 设置压缩模式失败:', error)
    }
}

/**
 * 设置压缩级别
 */
export async function setCompressionLevel(level: number): Promise<void> {
    try {
        if (await isTauriEnvironmentAvailable()) {
            const { invoke } = await import('@tauri-apps/api/core')
            await invoke('set_compression_level', { level })
        }
    } catch (error) {
        console.warn('[SettingsService] 设置压缩级别失败:', error)
    }
}
