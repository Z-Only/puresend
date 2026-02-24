/**
 * 分享链接状态管理
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
    ShareLinkInfo,
    AccessRequest,
    ShareSettings,
    FileMetadata,
    ContentType,
} from '../types'
import { DEFAULT_MAX_HISTORY_COUNT } from '../types/transfer'
import type { SelectedFileItem } from '../types/content'
import type { UnlistenFn } from '@tauri-apps/api/event'
import {
    startShareService,
    stopShareService,
    acceptAccessRequest,
    rejectAccessRequest,
    onAccessRequest,
    onAccessRequestAccepted,
    onAccessRequestRejected,
    onDownloadProgress,
    onDownloadComplete,
    onDownloadStart,
    type DownloadCompletePayload,
    type DownloadStartPayload,
} from '../services/shareService'
import { useSettingsStore } from './settings'

export const useShareStore = defineStore('share', () => {
    // ============ 状态 ============

    /** 当前分享链接信息 */
    const shareInfo = ref<ShareLinkInfo | null>(null)

    /** 访问请求列表 */
    const accessRequests = ref<Map<string, AccessRequest>>(new Map())

    /** 分享设置 */
    const settings = ref<ShareSettings>({
        pinEnabled: false,
        autoAccept: false,
    })

    /** 是否正在加载 */
    const loading = ref(false)

    /** 错误信息 */
    const error = ref<string>('')

    /** 二维码数据 URL */
    const qrCodeDataUrl = ref<string>('')

    /** 已选文件列表（用于链接分享） */
    const selectedFiles = ref<SelectedFileItem[]>([])

    /** 内容类型（发送页面的内容类型选择） */
    const contentType = ref<ContentType>('file')

    /** 事件监听器清理函数 */
    let unlistenFns: UnlistenFn[] = []

    // ============ 计算属性 ============

    /** 是否正在分享 */
    const isSharing = computed(() => shareInfo.value?.status === 'active')

    /** 分享链接 */
    const shareLink = computed(() => shareInfo.value?.link || '')

    /** 待处理的访问请求 */
    const pendingRequests = computed(() =>
        Array.from(accessRequests.value.values()).filter(
            (r) => r.status === 'pending'
        )
    )

    /** 已接受的访问请求 */
    const acceptedRequests = computed(() =>
        Array.from(accessRequests.value.values()).filter(
            (r) => r.status === 'accepted'
        )
    )

    /** 已拒绝的访问请求 */
    const rejectedRequests = computed(() =>
        Array.from(accessRequests.value.values()).filter(
            (r) => r.status === 'rejected'
        )
    )

    /** 待处理请求数量 */
    const pendingCount = computed(() => pendingRequests.value.length)

    // ============ 方法 ============

    /**
     * 处理访问请求事件
     */
    function handleAccessRequest(request: AccessRequest) {
        accessRequests.value.set(request.id, { ...request })
    }

    /**
     * 处理访问请求被接受事件
     */
    function handleAccessRequestAccepted(request: AccessRequest) {
        const existingRequest = accessRequests.value.get(request.id)
        if (existingRequest) {
            accessRequests.value.set(request.id, { ...request })
        }
    }

    /**
     * 处理访问请求被拒绝事件
     */
    function handleAccessRequestRejected(request: AccessRequest) {
        const existingRequest = accessRequests.value.get(request.id)
        if (existingRequest) {
            accessRequests.value.set(request.id, { ...request })
        }
    }

    /**
     * 处理下载进度事件
     */
    function handleDownloadProgress(progress: {
        downloadId: string
        fileName: string
        progress: number
        downloadedBytes: number
        totalBytes: number
        speed: number
        clientIp: string
    }) {
        // 更新对应访问请求的传输进度
        const request = Array.from(accessRequests.value.values()).find(
            (r) => r.ip === progress.clientIp
        )
        if (request) {
            accessRequests.value.set(request.id, {
                ...request,
                transferProgress: {
                    fileName: progress.fileName,
                    downloadedBytes: progress.downloadedBytes,
                    totalBytes: progress.totalBytes,
                    progress: progress.progress,
                    speed: progress.speed,
                    completedFiles: 0,
                    totalFiles: 1,
                    status: 'transferring',
                    startedAt: Date.now(),
                },
            })
        }
    }

    /**
     * 处理下载开始事件
     */
    function handleDownloadStart(payload: DownloadStartPayload) {
        console.log('下载开始:', payload)
        // 更新对应访问请求的传输进度状态
        const request = Array.from(accessRequests.value.values()).find(
            (r) => r.ip === payload.client_ip
        )
        if (request) {
            accessRequests.value.set(request.id, {
                ...request,
                transferProgress: {
                    fileName: payload.file_name,
                    downloadedBytes: 0,
                    totalBytes: payload.file_size,
                    progress: 0,
                    speed: 0,
                    completedFiles: 0,
                    totalFiles: shareInfo.value?.files.length || 1,
                    status: 'transferring',
                    startedAt: Date.now(),
                },
            })
        }
    }

    /**
     * 处理下载完成事件
     */
    async function handleDownloadComplete(payload: DownloadCompletePayload) {
        console.log('下载完成:', payload)

        // 检查是否需要记录历史
        const settingsStore = useSettingsStore()
        if (!settingsStore.history.recordHistory) {
            return
        }

        // 添加到传输历史记录
        const historyItem = {
            id: crypto.randomUUID(),
            fileName: payload.file_name,
            fileSize: payload.file_size,
            peerName: payload.client_ip,
            status: 'completed' as const,
            direction: 'send' as const,
            completedAt: Date.now(),
            mode: 'local' as const,
        }

        // 使用 transferStore 添加历史记录
        const { useTransferStore } = await import('./transfer')
        const transferStore = useTransferStore()

        // 直接添加到历史记录列表
        if (!transferStore.historyItems.some((h) => h.id === historyItem.id)) {
            transferStore.historyItems.unshift(historyItem)
            // 超出上限时移除最旧的记录
            if (transferStore.historyItems.length > DEFAULT_MAX_HISTORY_COUNT) {
                transferStore.historyItems = transferStore.historyItems.slice(
                    0,
                    DEFAULT_MAX_HISTORY_COUNT
                )
            }
            await transferStore.saveHistory()
        }
    }

    /**
     * 设置事件监听器
     */
    async function setupEventListeners(): Promise<void> {
        unlistenFns.push(
            await onAccessRequest(handleAccessRequest),
            await onAccessRequestAccepted(handleAccessRequestAccepted),
            await onAccessRequestRejected(handleAccessRequestRejected),
            await onDownloadProgress(handleDownloadProgress),
            await onDownloadStart(handleDownloadStart),
            await onDownloadComplete(handleDownloadComplete)
        )
    }

    /**
     * 开始分享
     * @param files 要分享的文件列表
     */
    async function startShare(
        files: FileMetadata[]
    ): Promise<ShareLinkInfo | null> {
        loading.value = true
        error.value = ''

        try {
            const result = await startShareService(files, settings.value)
            shareInfo.value = result

            // 设置事件监听
            await setupEventListeners()

            return result
        } catch (e) {
            error.value = `开始分享失败：${e}`
            console.error('开始分享失败:', e)
            return null
        } finally {
            loading.value = false
        }
    }

    /**
     * 停止分享
     */
    async function stopShare(): Promise<void> {
        loading.value = true
        error.value = ''

        try {
            await stopShareService()

            // 清理事件监听
            unlistenFns.forEach((unlisten) => unlisten())
            unlistenFns = []

            // 重置状态
            shareInfo.value = null
            accessRequests.value.clear()
        } catch (e) {
            error.value = `停止分享失败：${e}`
            console.error('停止分享失败:', e)
            throw e
        } finally {
            loading.value = false
        }
    }

    /**
     * 接受访问请求
     * @param requestId 请求 ID
     */
    async function acceptRequest(requestId: string): Promise<void> {
        const request = accessRequests.value.get(requestId)
        if (!request) return

        loading.value = true
        error.value = ''

        try {
            await acceptAccessRequest(requestId)

            // 更新请求状态
            accessRequests.value.set(requestId, {
                ...request,
                status: 'accepted',
            })
        } catch (e) {
            error.value = `接受请求失败：${e}`
            console.error('接受请求失败:', e)
            throw e
        } finally {
            loading.value = false
        }
    }

    /**
     * 拒绝访问请求
     * @param requestId 请求 ID
     */
    async function rejectRequest(requestId: string): Promise<void> {
        const request = accessRequests.value.get(requestId)
        if (!request) return

        loading.value = true
        error.value = ''

        try {
            await rejectAccessRequest(requestId)

            // 更新请求状态
            accessRequests.value.set(requestId, {
                ...request,
                status: 'rejected',
            })
        } catch (e) {
            error.value = `拒绝请求失败：${e}`
            console.error('拒绝请求失败:', e)
            throw e
        } finally {
            loading.value = false
        }
    }

    /**
     * 更新分享设置
     * @param newSettings 新设置
     */
    function updateSettings(newSettings: Partial<ShareSettings>): void {
        settings.value = { ...settings.value, ...newSettings }
    }

    /**
     * 重置 store 状态
     */
    function resetStore(): void {
        shareInfo.value = null
        accessRequests.value.clear()
        settings.value = {
            pinEnabled: false,
            autoAccept: false,
        }
        loading.value = false
        error.value = ''
        qrCodeDataUrl.value = ''
        selectedFiles.value = []
        contentType.value = 'file'
        unlistenFns = []
    }

    /**
     * 设置已选文件列表
     * @param files 文件列表
     */
    function setSelectedFiles(files: SelectedFileItem[]): void {
        selectedFiles.value = files
    }

    /**
     * 添加访问请求
     * @param request 访问请求
     */
    function addAccessRequest(request: AccessRequest): void {
        accessRequests.value.set(request.id, request)
    }

    /**
     * 设置二维码数据 URL
     * @param url 二维码数据 URL
     */
    function setQRCode(url: string): void {
        qrCodeDataUrl.value = url
    }

    /**
     * 清除所有访问请求
     */
    function clearRequests(): void {
        accessRequests.value.clear()
    }

    /**
     * 清空已选文件
     */
    function clearSelectedFiles(): void {
        selectedFiles.value = []
    }

    /**
     * 设置内容类型
     * @param type 内容类型
     */
    function setContentType(type: ContentType): void {
        contentType.value = type
    }

    /**
     * 销毁 store
     */
    function destroy(): void {
        unlistenFns.forEach((unlisten) => unlisten())
        unlistenFns = []
        shareInfo.value = null
        accessRequests.value.clear()
        qrCodeDataUrl.value = ''
        selectedFiles.value = []
        contentType.value = 'file'
    }

    return {
        // 状态
        shareInfo,
        accessRequests,
        settings,
        loading,
        error,
        qrCodeDataUrl,
        selectedFiles,
        contentType,
        // 计算属性
        isSharing,
        shareLink,
        pendingRequests,
        acceptedRequests,
        rejectedRequests,
        pendingCount,
        // 方法
        startShare,
        stopShare,
        acceptRequest,
        rejectRequest,
        updateSettings,
        resetStore,
        clearRequests,
        addAccessRequest,
        setQRCode,
        setSelectedFiles,
        clearSelectedFiles,
        setContentType,
        destroy,
    }
})
