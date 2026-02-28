/**
 * P2P 传输模块
 */

import type { Ref } from 'vue'
import type { FileMetadata, TransferTask, TransferMode } from '../../types'
import type { ResumableTaskInfo } from '../../services/transferService'
import {
    initTransfer,
    getTransferPort,
    prepareFileTransfer,
    sendFile,
    cancelTransfer,
    getActiveTasks,
    cleanupCompletedTasks,
    getResumableTasks as getResumableTasksService,
    resumeTransfer as resumeTransferService,
    cleanupResumeInfo as cleanupResumeInfoService,
    startReceiving as startReceivingService,
    stopReceiving as stopReceivingService,
    getNetworkInfo as getNetworkInfoService,
    setReceiveDirectory,
    getReceiveDirectory,
} from '../../services/transferService'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { setupP2PEventListeners } from './events'

// ============ P2P 传输相关函数 ============

/**
 * 初始化传输服务
 */
export async function initializeP2PTransfer(
    initialized: Ref<boolean>,
    listenPort: Ref<number>,
    receiveDirectory: Ref<string>,
    tasks: Ref<Map<string, TransferTask>>,
    loading: Ref<boolean>,
    error: Ref<string>,
    addHistoryItem: (taskOrItem: TransferTask) => Promise<void>
): Promise<UnlistenFn[]> {
    if (initialized.value) return []

    loading.value = true
    error.value = ''

    const unlistenFns: UnlistenFn[] = []

    try {
        // 初始化传输服务
        await initTransfer()

        // 获取监听端口
        listenPort.value = await getTransferPort()

        // 获取接收目录
        receiveDirectory.value = await getReceiveDirectory()

        // 同步现有任务
        const activeTasks = await getActiveTasks()
        tasks.value = new Map(activeTasks.map((t) => [t.id, t]))

        // 注册事件监听
        const eventListeners = await setupP2PEventListeners(
            tasks,
            addHistoryItem
        )
        unlistenFns.push(...eventListeners)

        initialized.value = true
    } catch (e) {
        error.value = `初始化失败：${e}`
        console.error('初始化传输服务失败:', e)
    } finally {
        loading.value = false
    }

    return unlistenFns
}

/**
 * 获取可恢复的传输任务列表
 */
export async function getResumableTasks(): Promise<ResumableTaskInfo[]> {
    try {
        return await getResumableTasksService()
    } catch (e) {
        console.error('[Transfer] 获取可恢复任务失败:', e)
        return []
    }
}

/**
 * 恢复中断的传输任务
 */
export async function resumeTransfer(
    tasks: Ref<Map<string, TransferTask>>,
    taskId: string
): Promise<void> {
    const task = tasks.value.get(taskId)
    if (task) {
        task.status = 'transferring'
        task.resumed = true
        task.error = undefined
    }

    try {
        await resumeTransferService(taskId)
    } catch (e) {
        if (task) {
            task.status = 'failed'
            task.error = `恢复传输失败：${e}`
        }
        console.error('[Transfer] 恢复传输失败:', e)
        throw e
    }
}

/**
 * 清理断点信息
 */
export async function cleanupResumeInfo(taskId?: string): Promise<void> {
    try {
        await cleanupResumeInfoService(taskId)
    } catch (e) {
        console.error('[Transfer] 清理断点信息失败:', e)
    }
}

/**
 * 准备文件传输
 */
export async function prepareTransfer(
    filePath: string,
    loading: Ref<boolean>,
    error: Ref<string>
): Promise<FileMetadata | null> {
    loading.value = true
    error.value = ''

    try {
        const metadata = await prepareFileTransfer(filePath)
        return metadata
    } catch (e) {
        error.value = `准备传输失败：${e}`
        console.error('准备文件传输失败:', e)
        return null
    } finally {
        loading.value = false
    }
}

/**
 * 发送文件
 */
export async function send(
    tasks: Ref<Map<string, TransferTask>>,
    selectedTaskId: Ref<string>,
    loading: Ref<boolean>,
    error: Ref<string>,
    fileMetadata: FileMetadata,
    peerId: string,
    peerIp: string,
    peerPort: number
): Promise<string | null> {
    loading.value = true
    error.value = ''

    try {
        const taskId = await sendFile(fileMetadata, peerId, peerIp, peerPort)

        // 创建新任务并添加到列表
        const task: TransferTask = {
            id: taskId,
            file: fileMetadata,
            mode: 'local',
            peer: {
                id: peerId,
                name: peerId,
                ip: peerIp,
                port: peerPort,
                deviceType: 'unknown',
                discoveredAt: Date.now(),
                lastSeen: Date.now(),
                status: 'available',
            },
            status: 'transferring',
            progress: 0,
            transferredBytes: 0,
            speed: 0,
            createdAt: Date.now(),
            direction: 'send',
        }

        tasks.value.set(taskId, task)
        selectedTaskId.value = taskId

        return taskId
    } catch (e) {
        error.value = `发送失败：${e}`
        console.error('发送文件失败:', e)
        return null
    } finally {
        loading.value = false
    }
}

/**
 * 获取网络信息（不启动接收服务）
 */
export async function getNetworkInfo(
    receivePort: Ref<number>,
    networkAddresses: Ref<string[]>,
    loading: Ref<boolean>,
    error: Ref<string>
): Promise<void> {
    loading.value = true
    error.value = ''

    try {
        const result = await getNetworkInfoService()
        receivePort.value = result.port
        // 后端使用 camelCase 序列化，直接访问
        networkAddresses.value = result.networkAddresses || []
    } catch (e) {
        error.value = `获取网络信息失败：${e}`
        console.error('获取网络信息失败:', e)
        throw e
    } finally {
        loading.value = false
    }
}

/**
 * 启动接收监听服务器
 */
export async function startReceiving(
    receivePort: Ref<number>,
    networkAddresses: Ref<string[]>,
    loading: Ref<boolean>,
    error: Ref<string>
): Promise<void> {
    loading.value = true
    error.value = ''

    try {
        const result = await startReceivingService()
        receivePort.value = result.port
        // 后端使用 camelCase 序列化，直接访问
        networkAddresses.value = result.networkAddresses || []
    } catch (e) {
        error.value = `启动接收失败：${e}`
        console.error('启动接收失败:', e)
        throw e
    } finally {
        loading.value = false
    }
}

/**
 * 停止接收监听服务器
 */
export async function stopReceiving(
    receivePort: Ref<number>,
    error: Ref<string>
): Promise<void> {
    try {
        await stopReceivingService()
        receivePort.value = 0
        // 保留 networkAddresses，停止接收时 IP 仍然可以显示
    } catch (e) {
        error.value = `停止接收失败：${e}`
        console.error('停止接收失败:', e)
        throw e
    }
}

/**
 * 设置接收目录
 */
export async function updateReceiveDirectory(
    receiveDirectory: Ref<string>,
    directory: string
): Promise<void> {
    try {
        await setReceiveDirectory(directory)
        receiveDirectory.value = directory
    } catch (e) {
        console.error('设置接收目录失败:', e)
        throw e
    }
}

/**
 * 取消传输
 */
export async function cancel(
    tasks: Ref<Map<string, TransferTask>>,
    taskId: string
): Promise<void> {
    try {
        await cancelTransfer(taskId)

        const task = tasks.value.get(taskId)
        if (task) {
            task.status = 'cancelled'
        }
    } catch (e) {
        console.error('取消传输失败:', e)
    }
}

/**
 * 清理已完成任务
 */
export async function cleanup(
    tasks: Ref<Map<string, TransferTask>>
): Promise<void> {
    try {
        await cleanupCompletedTasks()

        // 移除已完成的任务
        const completedIds: string[] = []
        for (const [id, task] of tasks.value.entries()) {
            if (task.status === 'completed' || task.status === 'cancelled') {
                completedIds.push(id)
            }
        }

        completedIds.forEach((id) => tasks.value.delete(id))
    } catch (e) {
        console.error('清理任务失败:', e)
    }
}

/**
 * 移除单个任务
 */
export async function removeTask(
    tasks: Ref<Map<string, TransferTask>>,
    taskId: string
): Promise<void> {
    try {
        const task = tasks.value.get(taskId)
        if (
            task &&
            (task.status === 'transferring' || task.status === 'pending')
        ) {
            await cancelTransfer(taskId)
        }
        tasks.value.delete(taskId)
    } catch (e) {
        console.error('移除任务失败:', e)
    }
}

/**
 * 设置传输模式
 */
export function setTransferMode(
    transferMode: Ref<TransferMode>,
    mode: TransferMode
): void {
    transferMode.value = mode
}

/**
 * 设置选中的目标设备 ID
 */
export function setSelectedPeerId(
    selectedPeerId: Ref<string>,
    peerId: string
): void {
    selectedPeerId.value = peerId
}

/**
 * 设置接收模式
 */
export function setReceiveMode(
    receiveMode: Ref<TransferMode>,
    mode: TransferMode
): void {
    receiveMode.value = mode
}

/**
 * 重置页面级状态（页面切换时调用，不影响 Web 服务状态和任务记录）
 */
export function resetPageState(
    transferMode: Ref<TransferMode>,
    selectedPeerId: Ref<string>,
    receiveMode: Ref<TransferMode>
): void {
    transferMode.value = 'local'
    selectedPeerId.value = ''
    receiveMode.value = 'local'
}
