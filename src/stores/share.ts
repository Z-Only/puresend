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
} from '../types'
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
} from '../services/shareService'

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
        // 可以在这里更新下载进度状态或通知用户
        console.log('下载进度:', progress)
    }

    /**
     * 设置事件监听器
     */
    async function setupEventListeners(): Promise<void> {
        unlistenFns.push(
            await onAccessRequest(handleAccessRequest),
            await onAccessRequestAccepted(handleAccessRequestAccepted),
            await onAccessRequestRejected(handleAccessRequestRejected),
            await onDownloadProgress(handleDownloadProgress)
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
     * 销毁 store
     */
    function destroy(): void {
        unlistenFns.forEach((unlisten) => unlisten())
        unlistenFns = []
        shareInfo.value = null
        accessRequests.value.clear()
        qrCodeDataUrl.value = ''
        selectedFiles.value = []
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
        destroy,
    }
})
