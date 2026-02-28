/**
 * 传输状态管理 - 聚合入口
 *
 * 此文件组合所有子模块，保持与原 transfer.ts 完全相同的 API
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
    FileMetadata,
    TransferTask,
    TransferMode,
    ReceiveTaskItem,
    SendTaskItem,
} from '../../types'
import type { UnlistenFn } from '@tauri-apps/api/event'

// ============ 导入子模块 ============

import * as historyModule from './history'
import * as webModule from './web'
import * as p2pModule from './p2p'

// ============ 定义 Store ============

export const useTransferStore = defineStore('transfer', () => {
    // ============ 状态 ============

    /** 是否已初始化 */
    const initialized = ref(false)

    /** 本机监听端口（发送） */
    const listenPort = ref<number>(0)

    /** 接收监听端口 */
    const receivePort = ref<number>(0)

    /** 本机网络地址 */
    const networkAddresses = ref<string[]>([])

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

    /** 传输模式（P2P 模式下的传输方式） */
    const transferMode = ref<TransferMode>('local')

    /** 选中的目标设备 ID（P2P 模式） */
    const selectedPeerId = ref<string>('')

    /** 接收模式 */
    const receiveMode = ref<TransferMode>('local')

    /** 事件监听器清理函数 */
    const unlistenFns: UnlistenFn[] = []

    // ============ 导入 Web 模块状态 ============

    const webUploadEnabled = webModule.webUploadEnabled
    const webUploadInfo = webModule.webUploadInfo
    const webUploadRequests = webModule.webUploadRequests
    const webDownloadEnabled = webModule.webDownloadEnabled
    const sendTaskItems = webModule.sendTaskItems
    const receiveTaskItems = webModule.receiveTaskItems

    // ============ 导入历史记录模块状态 ============

    const historyItems = historyModule.historyItems
    const historyLoaded = historyModule.historyLoaded
    const historyFilter = historyModule.historyFilter
    const historySort = historyModule.historySort

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

    /** Web 上传请求列表（数组形式） */
    const webUploadRequestList = computed(() =>
        Array.from(webUploadRequests.value.values()).sort(
            (a, b) => b.createdAt - a.createdAt
        )
    )

    /** 待处理的 Web 上传请求 */
    const pendingWebUploadRequests = computed(() =>
        webUploadRequestList.value.filter((r) => r.status === 'pending')
    )

    /** 统一接收任务列表（合并 P2P 接收任务和 Web 上传任务，按创建时间倒序） */
    const unifiedReceiveTasks = computed<ReceiveTaskItem[]>(() => {
        // 将 P2P 接收任务转换为 ReceiveTaskItem
        const p2pItems: ReceiveTaskItem[] = receiveTasks.value.map(
            (task): ReceiveTaskItem => ({
                id: `p2p-${task.id}`,
                source: 'p2p',
                senderLabel: task.peer?.name || task.peer?.ip || '',
                senderIp: task.peer?.ip || '',
                files: [
                    {
                        name: task.file.name,
                        size: task.file.size,
                        transferredBytes: task.transferredBytes,
                        progress: task.progress,
                        speed: task.speed,
                        status: task.status,
                        startedAt: task.createdAt,
                    },
                ],
                fileCount: 1,
                totalSize: task.file.size,
                totalTransferredBytes: task.transferredBytes,
                approvalStatus: 'accepted',
                transferStatus: task.status,
                progress: task.progress,
                speed: task.speed,
                createdAt: task.createdAt,
                completedAt: task.completedAt,
                error: task.error,
                originalTaskId: task.id,
            })
        )

        // 从 receiveTaskItems 获取 Web 上传任务
        const webItems = Array.from(receiveTaskItems.value.values())

        // 合并并按创建时间倒序排列
        return [...p2pItems, ...webItems].sort(
            (a, b) => b.createdAt - a.createdAt
        )
    })

    /** 待审批的统一接收任务 */
    const pendingReceiveTasks = computed(() =>
        unifiedReceiveTasks.value.filter(
            (task) => task.approvalStatus === 'pending'
        )
    )

    /** 统一发送任务列表（合并 P2P 发送任务和 Web 下载任务，按创建时间倒序） */
    const unifiedSendTasks = computed<SendTaskItem[]>(() => {
        // 将 P2P 发送任务转换为 SendTaskItem
        const p2pItems: SendTaskItem[] = sendTasks.value.map(
            (task): SendTaskItem => {
                // 根据 status 确定 approvalStatus
                let approvalStatus: 'pending' | 'accepted' | 'rejected'
                if (
                    task.status === 'transferring' ||
                    task.status === 'completed'
                ) {
                    approvalStatus = 'accepted'
                } else if (
                    task.status === 'failed' ||
                    task.status === 'cancelled'
                ) {
                    approvalStatus = 'rejected'
                } else {
                    approvalStatus = 'pending'
                }

                return {
                    id: `p2p-${task.id}`,
                    source: 'p2p',
                    receiverLabel: task.peer?.name || task.peer?.ip || '',
                    receiverIp: task.peer?.ip || '',
                    files: [
                        {
                            name: task.file.name,
                            size: task.file.size,
                            transferredBytes: task.transferredBytes,
                            progress: task.progress,
                            speed: task.speed,
                            status: task.status,
                            startedAt: task.createdAt,
                        },
                    ],
                    fileCount: 1,
                    totalSize: task.file.size,
                    totalTransferredBytes: task.transferredBytes,
                    approvalStatus,
                    transferStatus: task.status,
                    progress: task.progress,
                    speed: task.speed,
                    createdAt: task.createdAt,
                    completedAt: task.completedAt,
                    error: task.error,
                    originalTaskId: task.id,
                }
            }
        )

        // 从 sendTaskItems 获取 Web 下载任务
        const webItems = Array.from(sendTaskItems.value.values())

        // 合并并按创建时间倒序排列
        return [...p2pItems, ...webItems].sort(
            (a, b) => b.createdAt - a.createdAt
        )
    })

    /** 待审批的统一发送任务 */
    const pendingSendTasks = computed(() =>
        unifiedSendTasks.value.filter(
            (task) => task.approvalStatus === 'pending'
        )
    )

    // ============ 历史记录计算属性 ============

    const historyCount = historyModule.historyCount
    const filteredHistory = historyModule.filteredHistory
    const selectedHistoryItems = historyModule.selectedHistoryItems
    const selectedHistoryCount = historyModule.selectedHistoryCount

    // ============ 方法 ============

    /**
     * 初始化传输服务
     */
    async function initialize() {
        const p2pUnlistenFns = await p2pModule.initializeP2PTransfer(
            initialized,
            listenPort,
            receiveDirectory,
            tasks,
            loading,
            error,
            historyModule.addHistoryItem
        )
        unlistenFns.push(...p2pUnlistenFns)
    }

    /**
     * 准备文件传输
     */
    async function prepareTransfer(filePath: string) {
        return p2pModule.prepareTransfer(filePath, loading, error)
    }

    /**
     * 发送文件
     */
    async function send(
        fileMetadata: FileMetadata,
        peerId: string,
        peerIp: string,
        peerPort: number
    ) {
        return p2pModule.send(
            tasks,
            selectedTaskId,
            loading,
            error,
            fileMetadata,
            peerId,
            peerIp,
            peerPort
        )
    }

    /**
     * 获取网络信息（不启动接收服务）
     */
    async function getNetworkInfo() {
        return p2pModule.getNetworkInfo(
            receivePort,
            networkAddresses,
            loading,
            error
        )
    }

    /**
     * 启动接收监听服务器
     */
    async function startReceiving() {
        return p2pModule.startReceiving(
            receivePort,
            networkAddresses,
            loading,
            error
        )
    }

    /**
     * 停止接收监听服务器
     */
    async function stopReceiving() {
        return p2pModule.stopReceiving(receivePort, error)
    }

    /**
     * 设置接收目录
     */
    async function updateReceiveDirectory(directory: string) {
        return p2pModule.updateReceiveDirectory(receiveDirectory, directory)
    }

    /**
     * 取消传输
     */
    async function cancel(taskId: string) {
        return p2pModule.cancel(tasks, taskId)
    }

    /**
     * 清理已完成任务
     */
    async function cleanup() {
        return p2pModule.cleanup(tasks)
    }

    /**
     * 移除单个任务
     */
    async function removeTask(taskId: string) {
        return p2pModule.removeTask(tasks, taskId)
    }

    /**
     * 设置传输模式
     */
    function setTransferMode(mode: TransferMode) {
        return p2pModule.setTransferMode(transferMode, mode)
    }

    /**
     * 设置选中的目标设备 ID
     */
    function setSelectedPeerId(peerId: string) {
        return p2pModule.setSelectedPeerId(selectedPeerId, peerId)
    }

    /**
     * 设置接收模式
     */
    function setReceiveMode(mode: TransferMode) {
        return p2pModule.setReceiveMode(receiveMode, mode)
    }

    /**
     * 获取可恢复的传输任务列表
     */
    async function getResumableTasks() {
        return p2pModule.getResumableTasks()
    }

    /**
     * 恢复中断的传输任务
     */
    async function resumeTransfer(taskId: string) {
        return p2pModule.resumeTransfer(tasks, taskId)
    }

    /**
     * 清理断点信息
     */
    async function cleanupResumeInfo(taskId?: string) {
        return p2pModule.cleanupResumeInfo(taskId)
    }

    // ============ Web 上传方法 ============

    /**
     * 启动 Web 上传服务器
     */
    async function startWebUpload() {
        const webUnlistenFns = await webModule.startWebUpload(
            receiveDirectory,
            error
        )
        unlistenFns.push(...webUnlistenFns)
    }

    /**
     * 停止 Web 上传服务器
     */
    async function stopWebUpload() {
        return webModule.stopWebUpload()
    }

    /**
     * 同意 Web 上传请求
     */
    async function acceptWebUploadRequest(requestId: string) {
        return webModule.acceptWebUploadRequest(requestId)
    }

    /**
     * 拒绝 Web 上传请求
     */
    async function rejectWebUploadRequest(requestId: string) {
        return webModule.rejectWebUploadRequest(requestId)
    }

    // ============ 统一接收任务方法 ============

    /**
     * 同意统一接收任务
     */
    async function acceptReceiveTask(taskId: string) {
        return webModule.acceptReceiveTask(taskId)
    }

    /**
     * 拒绝统一接收任务
     */
    async function rejectReceiveTask(taskId: string) {
        return webModule.rejectReceiveTask(taskId, p2pModule.cancel, tasks)
    }

    /**
     * 清理已完成的统一接收任务
     */
    function cleanupReceiveTaskItems() {
        return webModule.cleanupReceiveTaskItems()
    }

    // ============ Web 模块辅助方法 ============

    const setSendTaskItem = webModule.setSendTaskItem
    const deleteSendTaskItem = webModule.deleteSendTaskItem
    const clearSendTaskItems = webModule.clearSendTaskItems
    const setReceiveTaskItem = webModule.setReceiveTaskItem
    const deleteReceiveTaskItem = webModule.deleteReceiveTaskItem
    const clearReceiveTaskItems = webModule.clearReceiveTaskItems

    // ============ 页面状态管理 ============

    /**
     * 重置页面级状态（页面切换时调用，不影响 Web 服务状态和任务记录）
     */
    function resetPageState() {
        return p2pModule.resetPageState(
            transferMode,
            selectedPeerId,
            receiveMode
        )
    }

    /**
     * 销毁全部服务状态（仅在应用退出时调用）
     */
    function destroy() {
        unlistenFns.forEach((unlisten) => unlisten())
        unlistenFns.length = 0
        tasks.value.clear()
        webModule.clearReceiveTaskItems()
        webModule.clearSendTaskItems()
        webDownloadEnabled.value = false
        webUploadEnabled.value = false
        webUploadInfo.value = null
        webUploadRequests.value.clear()
        initialized.value = false
        resetPageState()
    }

    // ============ 历史记录方法 ============

    const loadHistory = historyModule.loadHistory
    const saveHistory = historyModule.saveHistory
    const addHistoryItem = historyModule.addHistoryItem
    const removeHistoryItem = historyModule.removeHistoryItem
    const removeHistoryItems = historyModule.removeHistoryItems
    const clearHistory = historyModule.clearHistory
    const toggleHistorySelection = historyModule.toggleHistorySelection
    const toggleAllHistorySelection = historyModule.toggleAllHistorySelection
    const setHistoryFilter = historyModule.setHistoryFilter
    const setHistorySort = historyModule.setHistorySort
    const applyAutoCleanup = historyModule.applyAutoCleanup

    return {
        // ============ 状态 ============
        initialized,
        listenPort,
        receivePort,
        networkAddresses,
        receiveDirectory,
        tasks,
        selectedTaskId,
        loading,
        error,
        transferMode,
        selectedPeerId,
        receiveMode,

        // Web 上传
        webUploadEnabled,
        webUploadInfo,
        webUploadRequests,
        webUploadRequestList,
        pendingWebUploadRequests,
        startWebUpload,
        stopWebUpload,
        acceptWebUploadRequest,
        rejectWebUploadRequest,

        // Web 下载
        webDownloadEnabled,
        sendTaskItems,
        setSendTaskItem,
        deleteSendTaskItem,
        clearSendTaskItems,

        // 统一接收任务
        receiveTaskItems,
        setReceiveTaskItem,
        deleteReceiveTaskItem,
        clearReceiveTaskItems,
        unifiedReceiveTasks,
        pendingReceiveTasks,
        acceptReceiveTask,
        rejectReceiveTask,
        cleanupReceiveTaskItems,

        // 统一发送任务
        unifiedSendTasks,
        pendingSendTasks,

        // 历史记录状态
        historyItems,
        historyLoaded,
        historyFilter,
        historySort,

        // ============ 计算属性 ============
        taskList,
        sendTasks,
        receiveTasks,
        selectedTask,
        transferringTasks,
        completedTasks,
        failedTasks,
        isTransferring,

        // 历史记录计算属性
        historyCount,
        filteredHistory,
        selectedHistoryItems,
        selectedHistoryCount,

        // ============ 方法 ============
        initialize,
        prepareTransfer,
        send,
        getNetworkInfo,
        startReceiving,
        stopReceiving,
        updateReceiveDirectory,
        cancel,

        // 断点续传
        getResumableTasks,
        resumeTransfer,
        cleanupResumeInfo,
        cleanup,
        removeTask,
        setTransferMode,
        setSelectedPeerId,
        setReceiveMode,
        resetPageState,
        destroy,

        // 历史记录方法
        loadHistory,
        saveHistory,
        addHistoryItem,
        removeHistoryItem,
        removeHistoryItems,
        clearHistory,
        toggleHistorySelection,
        toggleAllHistorySelection,
        setHistoryFilter,
        setHistorySort,
        applyAutoCleanup,
    }
})

// 导出给 HistoryView 使用
if (typeof window !== 'undefined') {
    ;(
        window as { __TRANSFER_STORE__?: typeof useTransferStore }
    ).__TRANSFER_STORE__ = useTransferStore
}
