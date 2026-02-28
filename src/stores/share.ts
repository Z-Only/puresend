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
    UploadRecord,
    UploadProgress,
    SendTaskFileItem,
} from '../types'
import type { SelectedFileItem } from '../types/content'
import type { UnlistenFn } from '@tauri-apps/api/event'
import {
    startShareService,
    stopShareService,
    updateShareFilesService,
    acceptAccessRequest,
    rejectAccessRequest,
    removeAccessRequest,
    clearAccessRequests,
    onAccessRequest,
    onAccessRequestAccepted,
    onAccessRequestRejected,
    onAccessRequestRemoved,
    onUploadProgress,
    onUploadComplete,
    onUploadStart,
    type UploadCompletePayload,
    type UploadStartPayload,
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
    const unlistenFns: UnlistenFn[] = []

    // ============ 计算属性 ============

    /** 是否正在分享 */
    const isSharing = computed(() => shareInfo.value?.status === 'active')

    /** 分享链接 */
    const shareLinks = computed(() => shareInfo.value?.links || [])

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
     * 将访问请求同步到 transferStore.sendTaskItems
     */
    async function syncAccessRequestToSendTask(
        request: AccessRequest,
        approvalStatus: 'pending' | 'accepted' | 'rejected'
    ) {
        const { useTransferStore } = await import('./transfer/index')
        const transferStore = useTransferStore()
        const taskId = `web-${request.id}`

        const existingTask = transferStore.sendTaskItems.get(taskId)
        if (existingTask) {
            transferStore.setSendTaskItem(taskId, {
                ...existingTask,
                approvalStatus,
            })
        } else {
            transferStore.setSendTaskItem(taskId, {
                id: taskId,
                source: 'webDownload',
                receiverLabel: request.userAgent || '',
                receiverIp: request.ip,
                files: [],
                fileCount: 0,
                totalSize: 0,
                totalTransferredBytes: 0,
                approvalStatus,
                transferStatus: 'pending',
                progress: 0,
                speed: 0,
                createdAt: request.requestedAt,
                originalRequestId: request.id,
            })
        }
    }

    /**
     * 处理访问请求事件
     */
    async function handleAccessRequest(request: AccessRequest) {
        accessRequests.value.set(request.id, { ...request })
        await syncAccessRequestToSendTask(
            request,
            request.status as 'pending' | 'accepted' | 'rejected'
        )
    }

    /**
     * 处理访问请求被接受事件
     */
    async function handleAccessRequestAccepted(request: AccessRequest) {
        const existingRequest = accessRequests.value.get(request.id)
        if (existingRequest) {
            accessRequests.value.set(request.id, { ...request })
        }
        await syncAccessRequestToSendTask(request, 'accepted')
    }

    /**
     * 处理访问请求被拒绝事件
     */
    async function handleAccessRequestRejected(request: AccessRequest) {
        const existingRequest = accessRequests.value.get(request.id)
        if (existingRequest) {
            accessRequests.value.set(request.id, { ...request })
        }
        await syncAccessRequestToSendTask(request, 'rejected')
    }

    /**
     * 处理上传进度事件
     */
    async function handleUploadProgress(progress: UploadProgress) {
        for (const [requestId, request] of accessRequests.value.entries()) {
            const recordIndex = request.uploadRecords.findIndex(
                (r) => r.id === progress.uploadId
            )
            if (recordIndex !== -1) {
                const updatedRecords = [...request.uploadRecords]
                updatedRecords[recordIndex] = {
                    ...updatedRecords[recordIndex],
                    uploadedBytes: progress.uploadedBytes,
                    totalBytes: progress.totalBytes,
                    progress: progress.progress,
                    speed: progress.speed,
                    status: 'transferring',
                }
                accessRequests.value.set(requestId, {
                    ...request,
                    uploadRecords: updatedRecords,
                })

                // 同步到 sendTaskItems
                await syncUploadRecordToSendTask(
                    request.id,
                    progress.uploadId,
                    {
                        transferredBytes: progress.uploadedBytes,
                        progress: progress.progress,
                        speed: progress.speed,
                        status: 'transferring',
                    }
                )
                break
            }
        }
    }

    /**
     * 同步上传记录到 sendTaskItems 的 files 数组
     */
    async function syncUploadRecordToSendTask(
        requestId: string,
        uploadId: string,
        update: {
            transferredBytes?: number
            progress?: number
            speed?: number
            status?: string
            startedAt?: number
        }
    ) {
        const { useTransferStore } = await import('./transfer/index')
        const transferStore = useTransferStore()
        const taskId = `web-${requestId}`
        const task = transferStore.sendTaskItems.get(taskId)
        if (!task) return

        const fileIndex = task.files.findIndex((f) => f.uploadId === uploadId)
        if (fileIndex === -1) return

        const updatedFiles = [...task.files]
        updatedFiles[fileIndex] = {
            ...updatedFiles[fileIndex],
            ...(update.transferredBytes !== undefined && {
                transferredBytes: update.transferredBytes,
            }),
            ...(update.progress !== undefined && {
                progress: update.progress,
            }),
            ...(update.speed !== undefined && { speed: update.speed }),
            ...(update.status !== undefined && {
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                status: update.status as any,
            }),
            ...(update.startedAt !== undefined && {
                startedAt: update.startedAt,
            }),
        }

        // 计算整体进度
        const totalSize = updatedFiles.reduce((sum, f) => sum + f.size, 0)
        const totalTransferred = updatedFiles.reduce(
            (sum, f) => sum + f.transferredBytes,
            0
        )
        const overallProgress =
            totalSize > 0 ? Math.round((totalTransferred / totalSize) * 100) : 0
        const overallSpeed = updatedFiles.reduce(
            (sum, f) => sum + (f.status === 'transferring' ? f.speed : 0),
            0
        )
        const allCompleted = updatedFiles.every((f) => f.status === 'completed')

        transferStore.setSendTaskItem(taskId, {
            ...task,
            files: updatedFiles,
            fileCount: updatedFiles.length,
            totalSize,
            totalTransferredBytes: totalTransferred,
            progress: overallProgress,
            speed: overallSpeed,
            transferStatus: allCompleted ? 'completed' : 'transferring',
        })
    }

    /**
     * 处理上传开始事件
     */
    async function handleUploadStart(payload: UploadStartPayload) {
        console.log('上传开始:', payload)
        const request = Array.from(accessRequests.value.values()).find(
            (r) => r.ip === payload.client_ip
        )
        if (request) {
            const newRecord: UploadRecord = {
                id: payload.upload_id,
                fileName: payload.file_name,
                uploadedBytes: 0,
                totalBytes: payload.file_size,
                progress: 0,
                speed: 0,
                status: 'transferring',
                startedAt: Date.now(),
            }
            accessRequests.value.set(request.id, {
                ...request,
                uploadRecords: [newRecord, ...request.uploadRecords],
            })

            // 同步到 sendTaskItems 的 files 数组
            const { useTransferStore } = await import('./transfer/index')
            const transferStore = useTransferStore()
            const taskId = `web-${request.id}`
            const task = transferStore.sendTaskItems.get(taskId)
            if (task) {
                const newFileItem: SendTaskFileItem = {
                    name: payload.file_name,
                    size: payload.file_size,
                    transferredBytes: 0,
                    progress: 0,
                    speed: 0,
                    status: 'transferring',
                    startedAt: Date.now(),
                    uploadId: payload.upload_id,
                }
                transferStore.setSendTaskItem(taskId, {
                    ...task,
                    files: [newFileItem, ...task.files],
                    fileCount: task.files.length + 1,
                    totalSize: task.totalSize + payload.file_size,
                    transferStatus: 'transferring',
                })
            }
        }
    }

    /**
     * 处理上传完成事件
     */
    async function handleUploadComplete(payload: UploadCompletePayload) {
        console.log('上传完成:', payload)

        let matchedRequestId: string | null = null

        for (const [requestId, request] of accessRequests.value.entries()) {
            const recordIndex = request.uploadRecords.findIndex(
                (r) => r.id === payload.upload_id
            )
            if (recordIndex !== -1) {
                matchedRequestId = requestId
                const updatedRecords = [...request.uploadRecords]
                updatedRecords[recordIndex] = {
                    ...updatedRecords[recordIndex],
                    uploadedBytes: updatedRecords[recordIndex].totalBytes,
                    progress: 100,
                    speed: 0,
                    status: 'completed',
                    completedAt: Date.now(),
                }
                accessRequests.value.set(requestId, {
                    ...request,
                    uploadRecords: updatedRecords,
                })
                break
            }
        }

        // 同步到 sendTaskItems
        if (matchedRequestId) {
            await syncUploadRecordToSendTask(
                matchedRequestId,
                payload.upload_id,
                {
                    transferredBytes: payload.file_size,
                    progress: 100,
                    speed: 0,
                    status: 'completed',
                }
            )
        }

        // 检查是否需要记录历史
        const settingsStore = useSettingsStore()
        if (!settingsStore.history.recordHistory) {
            return
        }

        // 从匹配的 AccessRequest 中获取设备名称
        // userAgent 已由后端 parse_user_agent 解析为简短标识（如 "Chrome(Android)"），无需再次解析
        const matchedRequest = matchedRequestId
            ? accessRequests.value.get(matchedRequestId)
            : null
        const deviceLabel = matchedRequest?.userAgent || payload.client_ip

        // 添加到传输历史记录
        const historyItem = {
            id: crypto.randomUUID(),
            fileName: payload.file_name,
            fileSize: payload.file_size,
            peerName: deviceLabel,
            peerIp: payload.client_ip,
            status: 'completed' as const,
            direction: 'send' as const,
            completedAt: Date.now(),
            mode: 'local' as const,
        }

        // 直接调用 history 模块的公共方法添加历史记录
        const { addHistoryItem } = await import('./transfer/history')
        await addHistoryItem(historyItem)
    }

    /**
     * 处理访问请求被移除事件
     */
    function handleAccessRequestRemoved(requestId: string) {
        accessRequests.value.delete(requestId)
    }

    /**
     * 设置事件监听器
     */
    async function setupEventListeners(): Promise<void> {
        unlistenFns.push(
            await onAccessRequest(handleAccessRequest),
            await onAccessRequestAccepted(handleAccessRequestAccepted),
            await onAccessRequestRejected(handleAccessRequestRejected),
            await onAccessRequestRemoved(handleAccessRequestRemoved),
            await onUploadProgress(handleUploadProgress),
            await onUploadStart(handleUploadStart),
            await onUploadComplete(handleUploadComplete)
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
            const settingsStore = useSettingsStore()
            const preferredPort =
                settingsStore.webServerSettings.webDownloadLastPort || undefined
            const result = await startShareService(
                files,
                settings.value,
                preferredPort
            )
            shareInfo.value = result

            // 设置事件监听
            await setupEventListeners()

            // 持久化 Web 下载服务器状态
            await settingsStore.setWebDownloadState(true, result.port)

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
            unlistenFns.length = 0

            // 重置状态
            shareInfo.value = null
            accessRequests.value.clear()

            // 持久化 Web 下载服务器关闭状态（保留端口号）
            const settingsStore = useSettingsStore()
            await settingsStore.setWebDownloadState(false)
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
     * 移除单个访问请求
     * @param requestId 请求 ID
     */
    async function removeRequest(requestId: string): Promise<void> {
        const request = accessRequests.value.get(requestId)
        if (!request) return

        loading.value = true
        error.value = ''

        try {
            await removeAccessRequest(requestId)
            // 状态会在 handleAccessRequestRemoved 中自动移除
        } catch (e) {
            error.value = `移除请求失败：${e}`
            console.error('移除请求失败:', e)
            throw e
        } finally {
            loading.value = false
        }
    }

    /**
     * 移除所有访问请求
     */
    async function clearRequests(): Promise<void> {
        loading.value = true
        error.value = ''

        try {
            await clearAccessRequests()
            // 状态会在 handleAccessRequestRemoved 中自动移除
        } catch (e) {
            error.value = `清除请求失败：${e}`
            console.error('清除请求失败:', e)
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
        unlistenFns.length = 0
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
     * 更新分享文件列表（动态同步已选文件到后端 HTTP 服务器）
     * @param files 新的文件元数据列表
     */
    async function updateShareFiles(files: FileMetadata[]): Promise<void> {
        if (!isSharing.value) return

        try {
            await updateShareFilesService(files)
            // 同步更新本地 shareInfo 中的文件列表
            if (shareInfo.value) {
                shareInfo.value = {
                    ...shareInfo.value,
                    files,
                }
            }
        } catch (e) {
            console.error('更新分享文件列表失败:', e)
        }
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
        unlistenFns.length = 0
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
        shareLinks,
        pendingRequests,
        acceptedRequests,
        rejectedRequests,
        pendingCount,
        // 方法
        startShare,
        stopShare,
        updateShareFiles,
        acceptRequest,
        rejectRequest,
        removeRequest,
        clearRequests,
        updateSettings,
        resetStore,
        addAccessRequest,
        setQRCode,
        setSelectedFiles,
        clearSelectedFiles,
        setContentType,
        destroy,
    }
})
