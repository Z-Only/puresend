/**
 * Web 传输模块（Web 上传和 Web 下载）
 */

import { ref, triggerRef } from 'vue'
import type { Ref } from 'vue'
import type {
    WebUploadRequest,
    WebUploadInfo,
    WebUploadFileStartEvent,
    WebUploadFileProgressEvent,
    WebUploadFileCompleteEvent,
    ReceiveTaskItem,
    ReceiveTaskFileItem,
    SendTaskItem,
    TransferTask,
} from '../../types'
import type { UnlistenFn } from '@tauri-apps/api/event'
import {
    startWebUpload as startWebUploadService,
    stopWebUpload as stopWebUploadService,
    acceptWebUpload as acceptWebUploadService,
    rejectWebUpload as rejectWebUploadService,
    onWebUploadTask,
    onWebUploadStatusChanged,
    onWebUploadFileStart,
    onWebUploadFileProgress,
    onWebUploadFileComplete,
} from '../../services/webUploadService'
import {
    onNetworkChanged,
    updateLinkIp,
    type NetworkChangedPayload,
} from '../../services/networkService'
import { useSettingsStore } from '../settings'
import { addHistoryItem } from './history'
import type { TransferHistoryItem } from '../../types'

// ============ 网络变化监听 ============

let webUploadNetworkUnlisten: UnlistenFn | null = null

/**
 * 处理网络变化事件，更新 Web 上传链接
 */
function handleWebUploadNetworkChanged(payload: NetworkChangedPayload): void {
    if (payload.changeType === 'disconnected' || !webUploadInfo.value) return

    const updatedUrls = webUploadInfo.value.urls.map((url) =>
        updateLinkIp(url, payload.previousIpAddresses, payload.ipAddresses)
    )

    webUploadInfo.value = {
        ...webUploadInfo.value,
        urls: updatedUrls,
    }
}

// ============ Web 上传状态 ============

export const webUploadEnabled = ref(false)
export const webUploadInfo = ref<WebUploadInfo | null>(null)
export const webUploadRequests = ref<Map<string, WebUploadRequest>>(new Map())

// ============ Web 下载状态 ============

export const webDownloadEnabled = ref(false)
export const sendTaskItems = ref<Map<string, SendTaskItem>>(new Map())

// ============ 统一接收任务 ============

export const receiveTaskItems = ref<Map<string, ReceiveTaskItem>>(new Map())

// ============ Map 辅助方法（确保触发 Vue 响应式更新） ============

export function setSendTaskItem(key: string, value: SendTaskItem): void {
    sendTaskItems.value.set(key, value)
    triggerRef(sendTaskItems)
}

export function deleteSendTaskItem(key: string): void {
    sendTaskItems.value.delete(key)
    triggerRef(sendTaskItems)
}

export function clearSendTaskItems(): void {
    sendTaskItems.value.clear()
    triggerRef(sendTaskItems)
}

export function setReceiveTaskItem(key: string, value: ReceiveTaskItem): void {
    receiveTaskItems.value.set(key, value)
    triggerRef(receiveTaskItems)
}

export function deleteReceiveTaskItem(key: string): void {
    receiveTaskItems.value.delete(key)
    triggerRef(receiveTaskItems)
}

export function clearReceiveTaskItems(): void {
    receiveTaskItems.value.clear()
    triggerRef(receiveTaskItems)
}

// ============ Web 上传方法 ============

/**
 * 启动 Web 上传服务器
 */
export async function startWebUpload(
    receiveDirectory: Ref<string>,
    error: Ref<string>
): Promise<UnlistenFn[]> {
    const settingsStore = useSettingsStore()
    const unlistenFns: UnlistenFn[] = []

    try {
        const preferredPort =
            settingsStore.webServerSettings.webUploadLastPort || undefined
        const info = await startWebUploadService(
            receiveDirectory.value,
            settingsStore.receiveSettings.autoReceive,
            settingsStore.receiveSettings.fileOverwrite,
            preferredPort
        )
        webUploadInfo.value = info
        webUploadEnabled.value = true
        webUploadRequests.value.clear()

        // 注册 Web 上传事件监听
        unlistenFns.push(
            await onWebUploadTask(handleWebUploadTask),
            await onWebUploadStatusChanged(handleWebUploadStatusChanged),
            await onWebUploadFileStart(handleWebUploadFileStart),
            await onWebUploadFileProgress(handleWebUploadFileProgress),
            await onWebUploadFileComplete(handleWebUploadFileComplete)
        )

        // 启动网络变化监听，自动更新上传链接
        if (!webUploadNetworkUnlisten) {
            webUploadNetworkUnlisten = await onNetworkChanged(
                handleWebUploadNetworkChanged
            )
        }

        // 持久化 Web 上传服务器状态
        await settingsStore.setWebUploadState(true, info.port)
    } catch (e) {
        error.value = `启动 Web 上传失败：${e}`
        console.error('启动 Web 上传失败:', e)
        throw e
    }

    return unlistenFns
}

/**
 * 停止 Web 上传服务器
 */
export async function stopWebUpload(): Promise<void> {
    try {
        await stopWebUploadService()
        webUploadEnabled.value = false
        webUploadInfo.value = null
        webUploadRequests.value.clear()

        // 停止网络变化监听
        if (webUploadNetworkUnlisten) {
            webUploadNetworkUnlisten()
            webUploadNetworkUnlisten = null
        }

        // 持久化 Web 上传服务器关闭状态（保留端口号）
        const settingsStore = useSettingsStore()
        await settingsStore.setWebUploadState(false)
    } catch (e) {
        console.error('停止 Web 上传失败:', e)
        throw e
    }
}

/**
 * 同意 Web 上传请求
 */
export async function acceptWebUploadRequest(requestId: string): Promise<void> {
    try {
        await acceptWebUploadService(requestId)
    } catch (e) {
        console.error('同意上传请求失败:', e)
        throw e
    }
}

/**
 * 拒绝 Web 上传请求
 */
export async function rejectWebUploadRequest(requestId: string): Promise<void> {
    try {
        await rejectWebUploadService(requestId)
    } catch (e) {
        console.error('拒绝上传请求失败:', e)
        throw e
    }
}

// ============ Web 上传事件处理 ============

/**
 * 处理 Web 上传任务事件：按 IP 创建统一的 ReceiveTaskItem（初始无文件列表）
 */
function handleWebUploadTask(request: WebUploadRequest): void {
    webUploadRequests.value.set(request.id, request)

    // 如果该请求已有对应的 ReceiveTaskItem，则不重复创建
    if (receiveTaskItems.value.has(`web-${request.id}`)) {
        return
    }

    const isAccepted = request.status === 'accepted'
    const isRejected = request.status === 'rejected'

    const taskItem: ReceiveTaskItem = {
        id: `web-${request.id}`,
        source: 'webUpload',
        senderLabel: request.userAgent || request.clientIp,
        senderIp: request.clientIp,
        files: [],
        fileCount: 0,
        totalSize: 0,
        totalTransferredBytes: 0,
        approvalStatus: isAccepted
            ? 'accepted'
            : isRejected
              ? 'rejected'
              : 'pending',
        transferStatus: isAccepted ? 'transferring' : 'pending',
        progress: 0,
        speed: 0,
        createdAt: request.createdAt,
        originalRequestId: request.id,
    }

    setReceiveTaskItem(taskItem.id, taskItem)
}

/**
 * 处理 Web 上传状态变更事件：更新对应 ReceiveTaskItem 的审批状态
 */
function handleWebUploadStatusChanged(request: WebUploadRequest): void {
    webUploadRequests.value.set(request.id, request)

    const taskId = `web-${request.id}`
    const taskItem = receiveTaskItems.value.get(taskId)
    if (taskItem) {
        let approvalStatus = taskItem.approvalStatus
        let transferStatus = taskItem.transferStatus
        if (request.status === 'accepted') {
            approvalStatus = 'accepted'
            transferStatus = 'transferring'
        } else if (request.status === 'rejected') {
            approvalStatus = 'rejected'
            transferStatus = 'cancelled'
        } else if (request.status === 'expired') {
            approvalStatus = 'expired'
            transferStatus = 'cancelled'
        }
        setReceiveTaskItem(taskId, {
            ...taskItem,
            approvalStatus,
            transferStatus,
        })
    }
}

/**
 * 处理 Web 上传文件开始事件：向对应 ReceiveTaskItem 添加新文件条目
 */
function handleWebUploadFileStart(event: WebUploadFileStartEvent): void {
    const taskId = `web-${event.requestId}`
    const taskItem = receiveTaskItems.value.get(taskId)
    if (!taskItem) return

    const fileItem: ReceiveTaskFileItem = {
        name: event.fileName,
        size: event.totalBytes,
        transferredBytes: 0,
        progress: 0,
        speed: 0,
        status: 'transferring',
        startedAt: Date.now(),
    }

    const updatedFiles = [...taskItem.files, fileItem]
    setReceiveTaskItem(taskId, {
        ...taskItem,
        files: updatedFiles,
        fileCount: updatedFiles.length,
        totalSize: updatedFiles.reduce((sum, file) => sum + file.size, 0),
        transferStatus: 'transferring',
    })
}

/**
 * 处理 Web 上传文件进度事件：更新对应文件条目的进度
 */
function handleWebUploadFileProgress(event: WebUploadFileProgressEvent): void {
    const taskId = `web-${event.requestId}`
    const taskItem = receiveTaskItems.value.get(taskId)
    if (!taskItem) return

    const fileItem = findFileByRecord(taskItem, event.recordId, event.fileName)
    if (!fileItem) return

    const updatedFiles = taskItem.files.map((f) =>
        f === fileItem
            ? {
                  ...f,
                  transferredBytes: event.uploadedBytes,
                  progress: event.progress,
                  speed: event.speed,
                  size:
                      event.totalBytes > 0 && f.size === 0
                          ? event.totalBytes
                          : f.size,
              }
            : f
    )

    const totalTransferredBytes = updatedFiles.reduce(
        (sum, f) => sum + f.transferredBytes,
        0
    )
    const totalSize = updatedFiles.reduce((sum, f) => sum + f.size, 0)
    const progress =
        totalSize > 0
            ? Math.round((totalTransferredBytes / totalSize) * 100)
            : 0

    setReceiveTaskItem(taskId, {
        ...taskItem,
        files: updatedFiles,
        totalTransferredBytes,
        totalSize,
        progress,
        speed: event.speed,
    })
}

/**
 * 处理 Web 上传文件完成事件：标记对应文件条目为完成或失败
 */
function handleWebUploadFileComplete(event: WebUploadFileCompleteEvent): void {
    const taskId = `web-${event.requestId}`
    const taskItem = receiveTaskItems.value.get(taskId)
    if (!taskItem) return

    const fileItem = findFileByRecord(taskItem, event.recordId, event.fileName)

    const updatedFiles = taskItem.files.map((f) =>
        f === fileItem
            ? {
                  ...f,
                  status: (event.status === 'completed'
                      ? 'completed'
                      : 'failed') as ReceiveTaskFileItem['status'],
                  transferredBytes: event.totalBytes,
                  size: event.totalBytes > 0 ? event.totalBytes : f.size,
                  progress: event.status === 'completed' ? 100 : f.progress,
                  speed: 0,
              }
            : f
    )

    const totalTransferredBytes = updatedFiles.reduce(
        (sum, f) => sum + f.transferredBytes,
        0
    )
    const totalSize = updatedFiles.reduce((sum, f) => sum + f.size, 0)
    const progress =
        totalSize > 0
            ? Math.round((totalTransferredBytes / totalSize) * 100)
            : 0

    const allCompleted =
        updatedFiles.length > 0 &&
        updatedFiles.every(
            (file) => file.status === 'completed' || file.status === 'failed'
        )
    const hasFailure = updatedFiles.some((file) => file.status === 'failed')

    const completedAt = allCompleted ? Date.now() : taskItem.completedAt

    setReceiveTaskItem(taskId, {
        ...taskItem,
        files: updatedFiles,
        totalTransferredBytes,
        totalSize,
        progress,
        speed: 0,
        transferStatus: allCompleted
            ? hasFailure
                ? 'failed'
                : 'completed'
            : taskItem.transferStatus,
        completedAt,
    })

    // 所有文件传输完成后，将每个文件写入传输历史
    if (allCompleted) {
        const historyTimestamp = completedAt ?? Date.now()
        for (const file of updatedFiles) {
            const historyItem: TransferHistoryItem = {
                id: `${taskId}-${file.name}-${historyTimestamp}`,
                fileName: file.name,
                fileSize: file.size,
                peerName: taskItem.senderLabel,
                peerIp: taskItem.senderIp,
                status: file.status === 'completed' ? 'completed' : 'failed',
                direction: 'receive',
                completedAt: historyTimestamp,
            }
            addHistoryItem(historyItem)
        }
    }
}

// ============ recordId 到文件索引的映射缓存 ============

const recordIdToFileIndex = new Map<string, { taskId: string; index: number }>()

/**
 * 通过 recordId 和文件名查找 ReceiveTaskItem 中的文件条目。
 * 由于 ReceiveTaskFileItem 没有 recordId 字段，
 * 使用 recordId → 文件名映射缓存来精确匹配。
 */
function findFileByRecord(
    taskItem: ReceiveTaskItem,
    recordId: string,
    fileName: string
): ReceiveTaskFileItem | null {
    // 先从缓存中查找
    const cached = recordIdToFileIndex.get(recordId)
    if (
        cached &&
        cached.taskId === taskItem.id &&
        cached.index < taskItem.files.length
    ) {
        return taskItem.files[cached.index]
    }

    // 缓存未命中，按文件名查找最后一个正在传输的同名文件
    for (let i = taskItem.files.length - 1; i >= 0; i--) {
        if (
            taskItem.files[i].name === fileName &&
            taskItem.files[i].status === 'transferring'
        ) {
            recordIdToFileIndex.set(recordId, {
                taskId: taskItem.id,
                index: i,
            })
            return taskItem.files[i]
        }
    }

    // 兜底：按文件名查找最后一个匹配的文件
    for (let i = taskItem.files.length - 1; i >= 0; i--) {
        if (taskItem.files[i].name === fileName) {
            recordIdToFileIndex.set(recordId, {
                taskId: taskItem.id,
                index: i,
            })
            return taskItem.files[i]
        }
    }

    return null
}

// ============ 统一接收任务方法 ============

export async function acceptReceiveTask(taskId: string): Promise<void> {
    const taskItem = receiveTaskItems.value.get(taskId)
    if (taskItem?.source === 'webUpload' && taskItem.originalRequestId) {
        await acceptWebUploadRequest(taskItem.originalRequestId)
        return
    }
    // P2P 任务目前自动接受，无需额外操作
}

/**
 * 拒绝统一接收任务
 */
export async function rejectReceiveTask(
    taskId: string,
    cancel: (
        tasks: Ref<Map<string, TransferTask>>,
        taskId: string
    ) => Promise<void>,
    tasks: Ref<Map<string, TransferTask>>
): Promise<void> {
    const taskItem = receiveTaskItems.value.get(taskId)
    if (taskItem?.source === 'webUpload' && taskItem.originalRequestId) {
        await rejectWebUploadRequest(taskItem.originalRequestId)
        return
    }
    // P2P 任务：取消传输
    if (taskItem?.originalTaskId) {
        await cancel(tasks, taskItem.originalTaskId)
    }
}

/**
 * 清理已完成的统一接收任务
 */
export function cleanupReceiveTaskItems(): void {
    const idsToRemove: string[] = []
    for (const [id, task] of receiveTaskItems.value.entries()) {
        if (
            task.transferStatus === 'completed' ||
            task.transferStatus === 'cancelled' ||
            task.approvalStatus === 'rejected' ||
            task.approvalStatus === 'expired'
        ) {
            idsToRemove.push(id)
        }
    }
    idsToRemove.forEach((id) => receiveTaskItems.value.delete(id))
    if (idsToRemove.length > 0) {
        triggerRef(receiveTaskItems)
    }
}
