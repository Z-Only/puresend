/**
 * 云盘服务 - Tauri 命令封装
 *
 * 提供云盘相关的 Tauri API 调用封装，支持非 Tauri 环境降级
 */

import type {
    CloudAccount,
    CloudAccountCredentials,
    CloudAccountInput,
    CloudConnectionTestInput,
    CloudFileItem,
} from '@/types/cloud'
import { isTauriEnvironmentAvailable } from './settingsService'

// ============ 账号管理 ============

/**
 * 获取所有云盘账号列表
 */
export async function listCloudAccounts(): Promise<CloudAccount[]> {
    try {
        if (await isTauriEnvironmentAvailable()) {
            const { invoke } = await import('@tauri-apps/api/core')
            return await invoke<CloudAccount[]>('list_cloud_accounts')
        }
    } catch (error) {
        console.error('[CloudService] 获取云盘账号列表失败:', error)
        throw error
    }
    return []
}

/**
 * 添加云盘账号
 */
export async function addCloudAccount(
    input: CloudAccountInput
): Promise<CloudAccount> {
    if (await isTauriEnvironmentAvailable()) {
        const { invoke } = await import('@tauri-apps/api/core')
        return await invoke<CloudAccount>('add_cloud_account', { input })
    }
    throw new Error('Tauri 环境不可用')
}

/**
 * 更新云盘账号
 */
export async function updateCloudAccount(
    accountId: string,
    input: CloudAccountInput
): Promise<CloudAccount> {
    if (await isTauriEnvironmentAvailable()) {
        const { invoke } = await import('@tauri-apps/api/core')
        return await invoke<CloudAccount>('update_cloud_account', {
            accountId,
            input,
        })
    }
    throw new Error('Tauri 环境不可用')
}

/**
 * 删除云盘账号
 */
export async function deleteCloudAccount(accountId: string): Promise<void> {
    if (await isTauriEnvironmentAvailable()) {
        const { invoke } = await import('@tauri-apps/api/core')
        await invoke('delete_cloud_account', { accountId })
        return
    }
    throw new Error('Tauri 环境不可用')
}

// ============ 连接测试 ============

/**
 * 获取云盘账号凭证（用于编辑账号）
 */
export async function getCloudAccountCredentials(
    accountId: string
): Promise<CloudAccountCredentials> {
    if (await isTauriEnvironmentAvailable()) {
        const { invoke } = await import('@tauri-apps/api/core')
        return await invoke<CloudAccountCredentials>(
            'get_cloud_account_credentials',
            { accountId }
        )
    }
    throw new Error('Tauri 环境不可用')
}

/**
 * 测试已保存账号的连接
 */
export async function testCloudConnection(accountId: string): Promise<boolean> {
    if (await isTauriEnvironmentAvailable()) {
        const { invoke } = await import('@tauri-apps/api/core')
        return await invoke<boolean>('test_cloud_connection', { accountId })
    }
    throw new Error('Tauri 环境不可用')
}

/**
 * 使用临时凭证测试连接（用于添加账号前的验证）
 */
export async function testCloudConnectionWithCredentials(
    input: CloudConnectionTestInput
): Promise<boolean> {
    if (await isTauriEnvironmentAvailable()) {
        const { invoke } = await import('@tauri-apps/api/core')
        return await invoke<boolean>('test_cloud_connection_with_credentials', {
            input,
        })
    }
    throw new Error('Tauri 环境不可用')
}

// ============ 文件操作 ============

/**
 * 浏览云盘目录
 */
export async function browseCloudDirectory(
    accountId: string,
    path: string
): Promise<CloudFileItem[]> {
    if (await isTauriEnvironmentAvailable()) {
        const { invoke } = await import('@tauri-apps/api/core')
        return await invoke<CloudFileItem[]>('browse_cloud_directory', {
            accountId,
            path,
        })
    }
    throw new Error('Tauri 环境不可用')
}

/**
 * 创建云盘目录
 */
export async function createCloudDirectory(
    accountId: string,
    path: string
): Promise<void> {
    if (await isTauriEnvironmentAvailable()) {
        const { invoke } = await import('@tauri-apps/api/core')
        await invoke('create_cloud_directory', { accountId, path })
        return
    }
    throw new Error('Tauri 环境不可用')
}

/**
 * 上传文件到云盘
 */
export async function uploadToCloud(
    accountId: string,
    localPath: string,
    remotePath: string,
    overwrite: boolean = false
): Promise<void> {
    if (await isTauriEnvironmentAvailable()) {
        const { invoke } = await import('@tauri-apps/api/core')
        await invoke('upload_to_cloud', {
            accountId,
            localPath,
            remotePath,
            overwrite,
        })
        return
    }
    throw new Error('Tauri 环境不可用')
}

/**
 * 从云盘下载文件到本地
 */
export async function downloadFromCloud(
    accountId: string,
    remotePath: string,
    localPath: string
): Promise<void> {
    if (await isTauriEnvironmentAvailable()) {
        const { invoke } = await import('@tauri-apps/api/core')
        await invoke('download_from_cloud', {
            accountId,
            remotePath,
            localPath,
        })
        return
    }
    throw new Error('Tauri 环境不可用')
}
