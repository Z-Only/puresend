/**
 * 传输状态管理
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
    FileMetadata,
    TransferTask,
    TransferProgress,
    SendMode,
} from '../types'
import {
    initTransfer,
    getTransferPort,
    prepareFileTransfer,
    sendFile,
    cancelTransfer,
    getActiveTasks,
    cleanupCompletedTasks,
    onTransferProgress,
    onTransferError,
    onTransferComplete,
    startReceiving as startReceivingService,
    stopReceiving as stopReceivingService,
    getReceiveDirectory,
    setReceiveDirectory,
    getNetworkInfo as getNetworkInfoService,
} from '../services'
import type { UnlistenFn } from '@tauri-apps/api/event'

export const useTransferStore = defineStore('transfer', () => {
    // ============ 状态 ============

    /** 是否已初始化 */
    const initialized = ref(false)

    /** 本机监听端口（发送） */
    const listenPort = ref<number>(0)

    /** 接收监听端口 */
    const receivePort = ref<number>(0)

    /** 本机网络地址 */
    const networkAddress = ref<string>('')

    /** 分享码 */
    const shareCode = ref<string>('')

    /** 接收目录 */
    const receiveDirectory = ref<string>('~/Downloads/PureSend')

    /** 活跃的传输任务 */
    const tasks = ref<Map<string, TransferTask>>(new Map())

    /** 当前选中的任务 ID */
    const selectedTaskId = ref<string>('')

    /** 是否正在加载 */
    const loading = ref(false)

    /** 错误信息 */
    const error = ref<string>('')

    /** 发送模式（仅本地网络模式下可用） */
    const sendMode = ref<SendMode>('p2p')

    /** 事件监听器清理函数 */
    const unlistenFns: UnlistenFn[] = []

    // ============ 计算属性 ============

    /** 所有任务列表 */
    const taskList = computed(() => Array.from(tasks.value.values()))

    /** 发送任务列表 */
    const sendTasks = computed(() =>
        taskList.value.filter((t) => t.direction === 'send')
    )

    /** 接收任务列表 */
    const receiveTasks = computed(() =>
        taskList.value.filter((t) => t.direction === 'receive')
    )

    /** 当前选中的任务 */
    const selectedTask = computed(() => {
        if (!selectedTaskId.value) return null
        return tasks.value.get(selectedTaskId.value) || null
    })

    /** 正在传输的任务 */
    const transferringTasks = computed(() =>
        taskList.value.filter((t) => t.status === 'transferring')
    )

    /** 已完成的任务 */
    const completedTasks = computed(() =>
        taskList.value.filter((t) => t.status === 'completed')
    )

    /** 失败的任务 */
    const failedTasks = computed(() =>
        taskList.value.filter((t) => t.status === 'failed')
    )

    /** 是否有正在传输的任务 */
    const isTransferring = computed(() => transferringTasks.value.length > 0)

    // ============ 方法 ============

    /**
     * 初始化传输服务
     */
    async function initialize() {
        if (initialized.value) return

        loading.value = true
        error.value = ''

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
            unlistenFns.push(
                await onTransferProgress(handleProgress),
                await onTransferError(handleError),
                await onTransferComplete(handleComplete)
            )

            initialized.value = true
        } catch (e) {
            error.value = `初始化失败：${e}`
            console.error('初始化传输服务失败:', e)
        } finally {
            loading.value = false
        }
    }

    /**
     * 处理进度事件
     */
    function handleProgress(progress: TransferProgress) {
        const task = tasks.value.get(progress.taskId)
        if (task) {
            task.status = progress.status
            task.progress = progress.progress
            task.transferredBytes = progress.transferredBytes
            task.speed = progress.speed
        }
    }

    /**
     * 处理错误事件
     */
    function handleError(progress: TransferProgress) {
        const task = tasks.value.get(progress.taskId)
        if (task) {
            task.status = 'failed'
            task.error = progress.error
        }
    }

    /**
     * 处理完成事件
     */
    function handleComplete(progress: TransferProgress) {
        const task = tasks.value.get(progress.taskId)
        if (task) {
            task.status = 'completed'
            task.progress = 100
            task.transferredBytes = progress.totalBytes
        }
    }

    /**
     * 准备文件传输
     * @param filePath 文件路径
     */
    async function prepareTransfer(
        filePath: string
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
     * @param fileMetadata 文件元数据
     * @param peerId 目标设备 ID
     * @param peerIp 目标设备 IP
     * @param peerPort 目标设备端口
     */
    async function send(
        fileMetadata: FileMetadata,
        peerId: string,
        peerIp: string,
        peerPort: number
    ): Promise<string | null> {
        loading.value = true
        error.value = ''

        try {
            const taskId = await sendFile(
                fileMetadata,
                peerId,
                peerIp,
                peerPort
            )

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
    async function getNetworkInfo(): Promise<void> {
        loading.value = true
        error.value = ''

        try {
            const result = await getNetworkInfoService()
            receivePort.value = result.port
            // Tauri 返回的是 snake_case 字段，需要手动映射
            networkAddress.value = (result as any).network_address || ''
            shareCode.value = (result as any).share_code || ''
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
    async function startReceiving(): Promise<void> {
        loading.value = true
        error.value = ''

        try {
            const result = await startReceivingService()
            receivePort.value = result.port
            // Tauri 返回的是 snake_case 字段，需要手动映射
            networkAddress.value = (result as any).network_address || ''
            shareCode.value = (result as any).share_code || ''
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
    async function stopReceiving(): Promise<void> {
        try {
            await stopReceivingService()
            receivePort.value = 0
            networkAddress.value = ''
            shareCode.value = ''
        } catch (e) {
            error.value = `停止接收失败：${e}`
            console.error('停止接收失败:', e)
            throw e
        }
    }

    /**
     * 设置接收目录
     * @param directory 目录路径
     */
    async function updateReceiveDirectory(directory: string): Promise<void> {
        try {
            await setReceiveDirectory(directory)
            receiveDirectory.value = directory
        } catch (e) {
            error.value = `设置接收目录失败：${e}`
            console.error('设置接收目录失败:', e)
            throw e
        }
    }

    /**
     * 取消传输
     * @param taskId 任务 ID
     */
    async function cancel(taskId: string) {
        try {
            await cancelTransfer(taskId)

            const task = tasks.value.get(taskId)
            if (task) {
                task.status = 'cancelled'
            }
        } catch (e) {
            error.value = `取消失败：${e}`
            console.error('取消传输失败:', e)
        }
    }

    /**
     * 清理已完成任务
     */
    async function cleanup() {
        try {
            await cleanupCompletedTasks()

            // 移除已完成的任务
            const completedIds = Array.from(tasks.value.entries())
                .filter(
                    ([, task]) =>
                        task.status === 'completed' ||
                        task.status === 'cancelled'
                )
                .map(([id]) => id)

            completedIds.forEach((id) => tasks.value.delete(id))
        } catch (e) {
            error.value = `清理失败：${e}`
            console.error('清理任务失败:', e)
        }
    }

    /**
     * 移除单个任务
     * @param taskId 任务 ID
     */
    async function removeTask(taskId: string) {
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
            error.value = `移除任务失败：${e}`
            console.error('移除任务失败:', e)
        }
    }

    /**
     * 设置发送模式
     * @param mode 发送模式
     */
    function setSendMode(mode: SendMode): void {
        sendMode.value = mode
    }

    /**
     * 销毁 store，清理事件监听器
     */
    function destroy(): void {
        unlistenFns.forEach((unlisten) => unlisten())
        unlistenFns.length = 0
        tasks.value.clear()
        initialized.value = false
    }

    return {
        // 状态
        initialized,
        listenPort,
        receivePort,
        networkAddress,
        shareCode,
        receiveDirectory,
        tasks,
        selectedTaskId,
        loading,
        error,
        sendMode,
        // 计算属性
        taskList,
        sendTasks,
        receiveTasks,
        selectedTask,
        transferringTasks,
        completedTasks,
        failedTasks,
        isTransferring,
        // 方法
        initialize,
        prepareTransfer,
        send,
        getNetworkInfo,
        startReceiving,
        stopReceiving,
        updateReceiveDirectory,
        cancel,
        cleanup,
        removeTask,
        setSendMode,
        destroy,
    }
})
