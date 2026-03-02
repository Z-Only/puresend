/**
 * 云盘状态管理
 */
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
    CloudAccount,
    CloudAccountCredentials,
    CloudAccountInput,
    CloudConnectionTestInput,
    CloudFileItem,
    CloudUploadProgress,
    CloudDownloadProgress,
} from '@/types/cloud'
import type {
    TransferHistoryItem,
    TaskStatus,
    TransferDirection,
} from '@/types/transfer'
import {
    listCloudAccounts,
    addCloudAccount as addCloudAccountService,
    updateCloudAccount as updateCloudAccountService,
    deleteCloudAccount as deleteCloudAccountService,
    testCloudConnection as testCloudConnectionService,
    testCloudConnectionWithCredentials as testCloudConnectionWithCredentialsService,
    browseCloudDirectory as browseCloudDirectoryService,
    createCloudDirectory as createCloudDirectoryService,
    uploadToCloud as uploadToCloudService,
    downloadFromCloud as downloadFromCloudService,
    getCloudAccountCredentials as getCloudAccountCredentialsService,
} from '@/services/cloudService'
import { addHistoryItem } from '@/stores/transfer/history'

// ============ 存储键名 ============
const CLOUD_TASKS_STORAGE_KEY = 'cloud-tasks'
const CLOUD_TASKS_STORAGE_VERSION = 1

/** 云盘任务存储格式 */
interface CloudTasksStorage {
    version: number
    uploadTasks: CloudUploadTask[]
    downloadTasks: CloudDownloadTask[]
}

/** 上传任务 */
export interface CloudUploadTask {
    id: string
    accountId: string
    targetDirectory: string
    files: CloudUploadFileItem[]
    fileCount: number
    totalSize: number
    uploadedBytes: number
    progress: number
    status: 'pending' | 'uploading' | 'completed' | 'failed'
    createdAt: number
    completedAt?: number
    error?: string
}

/** 上传文件项 */
export interface CloudUploadFileItem {
    name: string
    localPath: string
    remotePath: string
    size: number
    uploadedBytes: number
    progress: number
    status: 'pending' | 'uploading' | 'completed' | 'failed'
    startedAt: number
    completedAt?: number
    error?: string
}

/** 下载任务 */
export interface CloudDownloadTask {
    id: string
    accountId: string
    accountName: string
    files: CloudDownloadFileItem[]
    fileCount: number
    totalSize: number
    totalTransferredBytes: number
    transferStatus: 'downloading' | 'completed' | 'failed'
    progress: number
    speed: number
    createdAt: number
    completedAt?: number
    error?: string
}

/** 下载文件项 */
export interface CloudDownloadFileItem {
    name: string
    size: number
    transferredBytes: number
    progress: number
    speed: number
    status: 'pending' | 'downloading' | 'completed' | 'failed'
    startedAt?: number
    completedAt?: number
    error?: string
}

export const useCloudStore = defineStore('cloud', () => {
    // ============ 状态 ============

    /** 云盘账号列表 */
    const accounts = ref<CloudAccount[]>([])

    /** 是否正在加载账号列表 */
    const loading = ref(false)

    /** 当前上传进度 */
    const uploadProgress = ref<CloudUploadProgress | null>(null)

    /** 当前下载进度 */
    const downloadProgress = ref<CloudDownloadProgress | null>(null)

    /** 上传任务列表 */
    const uploadTasks = ref<CloudUploadTask[]>([])

    /** 下载任务列表 */
    const downloadTasks = ref<CloudDownloadTask[]>([])

    // ============ 计算属性 ============

    /** 已连接的账号列表 */
    const connectedAccounts = computed(() =>
        accounts.value.filter((account) => account.status === 'connected')
    )

    /** 是否有可用的云盘账号 */
    const hasAccounts = computed(() => accounts.value.length > 0)

    /** 是否有已连接的云盘账号 */
    const hasConnectedAccounts = computed(
        () => connectedAccounts.value.length > 0
    )

    /** 进行中的上传任务 */
    const pendingUploadTasks = computed(() =>
        uploadTasks.value.filter((t) => t.status === 'uploading')
    )

    /** 进行中的下载任务 */
    const pendingDownloadTasks = computed(() =>
        downloadTasks.value.filter((t) => t.transferStatus === 'downloading')
    )

    // ============ 存储方法 ============

    /**
     * 从 localStorage 加载任务列表
     */
    function loadTasksFromStorage(): void {
        try {
            const data = localStorage.getItem(CLOUD_TASKS_STORAGE_KEY)
            if (!data) return

            const parsed = JSON.parse(data) as CloudTasksStorage
            if (parsed.version !== CLOUD_TASKS_STORAGE_VERSION) {
                // 版本不匹配，清空存储
                localStorage.removeItem(CLOUD_TASKS_STORAGE_KEY)
                return
            }

            // 加载所有任务（包括已完成和失败的），直到应用关闭前都保留
            uploadTasks.value = parsed.uploadTasks || []
            downloadTasks.value = (parsed.downloadTasks ||
                []) as CloudDownloadTask[]
        } catch (error) {
            console.error('[CloudStore] 加载任务列表失败:', error)
        }
    }

    /**
     * 保存任务列表到 localStorage
     */
    function saveTasksToStorage(): void {
        try {
            const storageData: CloudTasksStorage = {
                version: CLOUD_TASKS_STORAGE_VERSION,
                uploadTasks: uploadTasks.value,
                downloadTasks: downloadTasks.value,
            }
            localStorage.setItem(
                CLOUD_TASKS_STORAGE_KEY,
                JSON.stringify(storageData)
            )
        } catch (error) {
            console.error('[CloudStore] 保存任务列表失败:', error)
        }
    }

    /**
     * 同步上传任务到传输历史
     */
    async function syncUploadTaskToHistory(
        task: CloudUploadTask
    ): Promise<void> {
        // 为每个文件创建历史记录
        for (const file of task.files) {
            if (file.status !== 'completed' && file.status !== 'failed')
                continue

            const account = getAccountById(task.accountId)
            const historyItem: TransferHistoryItem = {
                id: `cloud-upload-${task.id}-${file.localPath}`,
                fileName: file.name,
                fileSize: file.size,
                peerName: account?.name || task.accountId,
                peerIp: undefined, // 云盘传输没有 IP
                status: file.status as TaskStatus,
                direction: 'send' as TransferDirection,
                completedAt: file.completedAt || Date.now(),
                mode: 'cloud',
                error: file.error,
            }
            await addHistoryItem(historyItem)
        }
    }

    /**
     * 同步下载任务到传输历史
     */
    async function syncDownloadTaskToHistory(
        task: CloudDownloadTask
    ): Promise<void> {
        // 为每个文件创建历史记录
        for (const file of task.files) {
            // 只处理已完成或失败的文件
            if (file.status !== 'completed' && file.status !== 'failed') {
                continue
            }

            const historyItem: TransferHistoryItem = {
                id: `cloud-download-${task.id}-${file.name}`,
                fileName: file.name,
                fileSize: file.size,
                peerName: task.accountName,
                peerIp: undefined, // 云盘传输没有 IP
                status: file.status as TaskStatus,
                direction: 'receive' as TransferDirection,
                completedAt: file.completedAt || Date.now(),
                mode: 'cloud',
                error: file.error,
            }
            await addHistoryItem(historyItem)
        }
    }

    // ============ 方法 ============

    /**
     * 加载云盘账号列表
     */
    async function loadAccounts(): Promise<void> {
        loading.value = true
        try {
            accounts.value = await listCloudAccounts()
        } catch (error) {
            console.error('[CloudStore] 加载云盘账号失败:', error)
            accounts.value = []
        } finally {
            loading.value = false
        }
    }

    /**
     * 添加云盘账号
     */
    async function addAccount(input: CloudAccountInput): Promise<CloudAccount> {
        const account = await addCloudAccountService(input)
        accounts.value.push(account)
        return account
    }

    /**
     * 更新云盘账号
     */
    async function updateAccount(
        accountId: string,
        input: CloudAccountInput
    ): Promise<CloudAccount> {
        const updated = await updateCloudAccountService(accountId, input)
        const index = accounts.value.findIndex((a) => a.id === accountId)
        if (index !== -1) {
            accounts.value[index] = updated
        }
        return updated
    }

    /**
     * 删除云盘账号
     */
    async function deleteAccount(accountId: string): Promise<void> {
        await deleteCloudAccountService(accountId)
        accounts.value = accounts.value.filter((a) => a.id !== accountId)
    }

    /**
     * 测试已保存账号的连接
     */
    async function testConnection(accountId: string): Promise<boolean> {
        const result = await testCloudConnectionService(accountId)
        // 更新本地账号状态
        const account = accounts.value.find((a) => a.id === accountId)
        if (account) {
            account.status = result ? 'connected' : 'invalid'
        }
        return result
    }

    /**
     * 使用临时凭证测试连接
     */
    async function testConnectionWithCredentials(
        input: CloudConnectionTestInput
    ): Promise<boolean> {
        return await testCloudConnectionWithCredentialsService(input)
    }

    /**
     * 浏览云盘目录
     */
    async function browseDirectory(
        accountId: string,
        path: string
    ): Promise<CloudFileItem[]> {
        return await browseCloudDirectoryService(accountId, path)
    }

    /**
     * 创建云盘目录
     */
    async function createDirectory(
        accountId: string,
        path: string
    ): Promise<void> {
        await createCloudDirectoryService(accountId, path)
    }

    /**
     * 上传文件到云盘
     */
    async function uploadFile(
        accountId: string,
        localPath: string,
        remotePath: string,
        overwrite: boolean = false
    ): Promise<void> {
        await uploadToCloudService(accountId, localPath, remotePath, overwrite)
    }

    /**
     * 从云盘下载文件到本地
     */
    async function downloadFile(
        accountId: string,
        remotePath: string,
        localPath: string
    ): Promise<void> {
        await downloadFromCloudService(accountId, remotePath, localPath)
    }

    /**
     * 更新上传进度
     */
    function setUploadProgress(progress: CloudUploadProgress | null): void {
        uploadProgress.value = progress
    }

    /**
     * 更新下载进度
     */
    function setDownloadProgress(progress: CloudDownloadProgress | null): void {
        downloadProgress.value = progress
    }

    /**
     * 根据 ID 获取账号
     */
    function getAccountById(accountId: string): CloudAccount | undefined {
        return accounts.value.find((a) => a.id === accountId)
    }

    /**
     * 获取账号凭证（用于编辑账号）
     */
    async function getAccountCredentials(
        accountId: string
    ): Promise<CloudAccountCredentials> {
        return await getCloudAccountCredentialsService(accountId)
    }

    /**
     * 添加或更新上传任务
     */
    function setUploadTask(task: CloudUploadTask): void {
        const index = uploadTasks.value.findIndex((t) => t.id === task.id)
        if (index !== -1) {
            uploadTasks.value[index] = task
        } else {
            uploadTasks.value.unshift(task)
        }
        saveTasksToStorage()

        // 如果任务已完成或失败，同步到历史记录
        if (task.status === 'completed' || task.status === 'failed') {
            syncUploadTaskToHistory(task)
        }
    }

    /**
     * 添加或更新下载任务
     */
    function setDownloadTask(task: CloudDownloadTask): void {
        const index = downloadTasks.value.findIndex((t) => t.id === task.id)
        if (index !== -1) {
            downloadTasks.value[index] = task
        } else {
            downloadTasks.value.unshift(task)
        }
        saveTasksToStorage()

        // 如果任务已完成或失败，同步到历史记录
        if (
            task.transferStatus === 'completed' ||
            task.transferStatus === 'failed'
        ) {
            syncDownloadTaskToHistory(task)
        }
    }

    /**
     * 移除上传任务
     */
    function removeUploadTask(taskId: string): void {
        const index = uploadTasks.value.findIndex((t) => t.id === taskId)
        if (index !== -1) {
            uploadTasks.value.splice(index, 1)
            saveTasksToStorage()
        }
    }

    /**
     * 移除下载任务
     */
    function removeDownloadTask(taskId: string): void {
        const index = downloadTasks.value.findIndex((t) => t.id === taskId)
        if (index !== -1) {
            downloadTasks.value.splice(index, 1)
            saveTasksToStorage()
        }
    }

    /**
     * 清理已完成的云盘任务
     */
    function cleanupCompletedTasks(): void {
        uploadTasks.value = uploadTasks.value.filter(
            (t) => t.status !== 'completed' && t.status !== 'failed'
        )
        downloadTasks.value = downloadTasks.value.filter(
            (t) =>
                t.transferStatus !== 'completed' &&
                t.transferStatus !== 'failed'
        )
        saveTasksToStorage()
    }

    /**
     * 初始化云盘 store
     */
    async function initialize(): Promise<void> {
        await loadAccounts()
        loadTasksFromStorage()
    }

    return {
        // 状态
        accounts,
        loading,
        uploadProgress,
        downloadProgress,
        uploadTasks,
        downloadTasks,

        // 计算属性
        connectedAccounts,
        hasAccounts,
        hasConnectedAccounts,
        pendingUploadTasks,
        pendingDownloadTasks,

        // 方法
        loadAccounts,
        addAccount,
        updateAccount,
        deleteAccount,
        testConnection,
        testConnectionWithCredentials,
        browseDirectory,
        createDirectory,
        uploadFile,
        downloadFile,
        setUploadProgress,
        setDownloadProgress,
        getAccountById,
        getAccountCredentials,
        setUploadTask,
        setDownloadTask,
        removeUploadTask,
        removeDownloadTask,
        cleanupCompletedTasks,
        initialize,
    }
})
