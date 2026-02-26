/**
 * 传输状态管理
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
    FileMetadata,
    TransferTask,
    TransferProgress,
    TransferMode,
    TransferHistoryItem,
    HistoryFilter,
    HistorySortOption,
    TransferHistoryStorage,
    WebUploadRequest,
    WebUploadInfo,
    WebUploadFileStartEvent,
    WebUploadFileProgressEvent,
    WebUploadFileCompleteEvent,
    ReceiveTaskItem,
    ReceiveTaskFileItem,
    SendTaskItem,
    SendTaskFileItem,
} from '../types'
import {
    HISTORY_STORAGE_VERSION,
    HISTORY_STORAGE_KEY,
    DEFAULT_MAX_HISTORY_COUNT,
} from '../types/transfer'
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
    startWebUpload as startWebUploadService,
    stopWebUpload as stopWebUploadService,
    acceptWebUpload as acceptWebUploadService,
    rejectWebUpload as rejectWebUploadService,
    onWebUploadTask,
    onWebUploadStatusChanged,
    onWebUploadFileStart,
    onWebUploadFileProgress,
    onWebUploadFileComplete,
} from '../services'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { useSettingsStore } from './settings'

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

    // ============ Web 上传状态 ============

    /** Web 上传是否已启用 */
    const webUploadEnabled = ref(false)

    /** Web 上传服务器信息 */
    const webUploadInfo = ref<WebUploadInfo | null>(null)

    /** Web 上传请求列表 */
    const webUploadRequests = ref<Map<string, WebUploadRequest>>(new Map())

    /** 统一接收任务列表（包含 P2P 和 Web 上传） */
    const receiveTaskItems = ref<Map<string, ReceiveTaskItem>>(new Map())

    /** Web 下载是否已启用 */
    const webDownloadEnabled = ref(false)

    /** 统一发送任务列表（Web 下载部分） */
    const sendTaskItems = ref<Map<string, SendTaskItem>>(new Map())

    /** 事件监听器清理函数 */
    const unlistenFns: UnlistenFn[] = []

    // ============ 历史记录状态 ============

    /** 历史记录列表 */
    const historyItems = ref<TransferHistoryItem[]>([])

    /** 历史记录是否已加载 */
    const historyLoaded = ref(false)

    /** 历史记录筛选条件 */
    const historyFilter = ref<HistoryFilter>({
        direction: 'all',
        status: 'all',
    })

    /** 历史记录排序选项 */
    const historySort = ref<HistorySortOption>({
        field: 'completedAt',
        order: 'desc',
    })

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
                        } as SendTaskFileItem,
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

    /** 历史记录总数量 */
    const historyCount = computed(() => historyItems.value.length)

    /** 筛选后的历史记录 */
    const filteredHistory = computed(() => {
        let result = [...historyItems.value]

        // 筛选方向
        if (
            historyFilter.value.direction &&
            historyFilter.value.direction !== 'all'
        ) {
            result = result.filter(
                (item) => item.direction === historyFilter.value.direction
            )
        }

        // 筛选状态
        if (
            historyFilter.value.status &&
            historyFilter.value.status !== 'all'
        ) {
            result = result.filter(
                (item) => item.status === historyFilter.value.status
            )
        }

        // 排序
        result.sort((a, b) => {
            const field = historySort.value.field
            const order = historySort.value.order
            let comparison = 0

            if (field === 'completedAt') {
                comparison = a.completedAt - b.completedAt
            } else if (field === 'fileName') {
                comparison = a.fileName.localeCompare(b.fileName)
            } else if (field === 'fileSize') {
                comparison = a.fileSize - b.fileSize
            }

            return order === 'desc' ? -comparison : comparison
        })

        return result
    })

    /** 选中的历史记录 */
    const selectedHistoryItems = computed(() =>
        historyItems.value.filter((item) => item.selected)
    )

    /** 选中的历史记录数量 */
    const selectedHistoryCount = computed(
        () => selectedHistoryItems.value.length
    )

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
            task.completedAt = Date.now()

            // 检查是否需要记录历史
            const settingsStore = useSettingsStore()
            if (settingsStore.history.recordHistory) {
                // 自动添加到历史记录
                addHistoryItem(task)
            }
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
            // 后端使用 camelCase 序列化，直接访问
            networkAddress.value = result.networkAddress || ''
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
            // 后端使用 camelCase 序列化，直接访问
            networkAddress.value = result.networkAddress || ''
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
            // 保留 networkAddress，停止接收时 IP 仍然可以显示
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
     * 设置传输模式
     * @param mode 传输模式
     */
    function setTransferMode(mode: TransferMode): void {
        transferMode.value = mode
    }

    /**
     * 设置选中的目标设备 ID
     * @param peerId 设备 ID
     */
    function setSelectedPeerId(peerId: string): void {
        selectedPeerId.value = peerId
    }

    /**
     * 设置接收模式
     * @param mode 接收模式
     */
    function setReceiveMode(mode: TransferMode): void {
        receiveMode.value = mode
    }

    // ============ Web 上传方法 ============

    /**
     * 启动 Web 上传服务器
     */
    async function startWebUpload(): Promise<void> {
        const settingsStore = useSettingsStore()
        try {
            const info = await startWebUploadService(
                receiveDirectory.value,
                settingsStore.receiveSettings.autoReceive,
                settingsStore.receiveSettings.fileOverwrite
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
        } catch (e) {
            error.value = `启动 Web 上传失败：${e}`
            console.error('启动 Web 上传失败:', e)
            throw e
        }
    }

    /**
     * 停止 Web 上传服务器
     */
    async function stopWebUpload(): Promise<void> {
        try {
            await stopWebUploadService()
            webUploadEnabled.value = false
            webUploadInfo.value = null
            webUploadRequests.value.clear()
        } catch (e) {
            error.value = `停止 Web 上传失败：${e}`
            console.error('停止 Web 上传失败:', e)
            throw e
        }
    }

    /**
     * 同意 Web 上传请求
     */
    async function acceptWebUploadRequest(requestId: string): Promise<void> {
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
    async function rejectWebUploadRequest(requestId: string): Promise<void> {
        try {
            await rejectWebUploadService(requestId)
        } catch (e) {
            console.error('拒绝上传请求失败:', e)
            throw e
        }
    }

    /**
     * 从 User-Agent 字符串中解析出浏览器名称和操作系统，
     * 返回类似 "Chrome(Android)" 的简短设备标识
     */
    function parseUserAgent(userAgent?: string): string {
        if (!userAgent) return ''

        const browser = detectBrowser(userAgent)
        const operatingSystem = detectOperatingSystem(userAgent)
        return operatingSystem ? `${browser}(${operatingSystem})` : browser
    }

    function detectBrowser(userAgent: string): string {
        if (userAgent.includes('Edg/') || userAgent.includes('Edge/')) {
            return 'Edge'
        }
        if (userAgent.includes('Chrome/') && !userAgent.includes('Edg/')) {
            return 'Chrome'
        }
        if (userAgent.includes('Safari/') && !userAgent.includes('Chrome/')) {
            return 'Safari'
        }
        if (userAgent.includes('Firefox/')) {
            return 'Firefox'
        }
        if (userAgent.includes('Opera/') || userAgent.includes('OPR/')) {
            return 'Opera'
        }
        return 'Browser'
    }

    function detectOperatingSystem(userAgent: string): string {
        if (userAgent.includes('Android')) {
            return 'Android'
        }
        if (userAgent.includes('iPhone') || userAgent.includes('iPad')) {
            return 'iOS'
        }
        if (userAgent.includes('Mac OS')) {
            return 'macOS'
        }
        if (userAgent.includes('Windows')) {
            return 'Windows'
        }
        if (userAgent.includes('Linux')) {
            return 'Linux'
        }
        return ''
    }

    /**
     * 处理 Web 上传任务事件：按 IP 创建统一的 ReceiveTaskItem（初始无文件列表）
     */
    function handleWebUploadTask(request: WebUploadRequest) {
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
            senderLabel: parseUserAgent(request.userAgent) || request.clientIp,
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

        receiveTaskItems.value.set(taskItem.id, taskItem)
    }

    /**
     * 处理 Web 上传状态变更事件：更新对应 ReceiveTaskItem 的审批状态
     */
    function handleWebUploadStatusChanged(request: WebUploadRequest) {
        webUploadRequests.value.set(request.id, request)

        const taskItem = receiveTaskItems.value.get(`web-${request.id}`)
        if (taskItem) {
            if (request.status === 'accepted') {
                taskItem.approvalStatus = 'accepted'
                taskItem.transferStatus = 'transferring'
            } else if (request.status === 'rejected') {
                taskItem.approvalStatus = 'rejected'
                taskItem.transferStatus = 'cancelled'
            } else if (request.status === 'expired') {
                taskItem.approvalStatus = 'expired'
                taskItem.transferStatus = 'cancelled'
            }
        }
    }

    /**
     * 处理 Web 上传文件开始事件：向对应 ReceiveTaskItem 添加新文件条目
     */
    function handleWebUploadFileStart(event: WebUploadFileStartEvent) {
        const taskItem = receiveTaskItems.value.get(`web-${event.requestId}`)
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

        taskItem.files.push(fileItem)
        taskItem.fileCount = taskItem.files.length
        taskItem.totalSize = taskItem.files.reduce(
            (sum, file) => sum + file.size,
            0
        )
        taskItem.transferStatus = 'transferring'
    }

    /**
     * 处理 Web 上传文件进度事件：更新对应文件条目的进度
     */
    function handleWebUploadFileProgress(event: WebUploadFileProgressEvent) {
        const taskItem = receiveTaskItems.value.get(`web-${event.requestId}`)
        if (!taskItem) return

        // 通过 recordId 匹配文件（recordId 存储在文件名旁，用文件名 + 顺序匹配）
        // 由于 ReceiveTaskFileItem 没有 recordId 字段，按文件名匹配最后一个同名文件
        const fileItem = findFileByRecord(
            taskItem,
            event.recordId,
            event.fileName
        )
        if (!fileItem) return

        fileItem.transferredBytes = event.uploadedBytes
        fileItem.progress = event.progress
        fileItem.speed = event.speed
        if (event.totalBytes > 0 && fileItem.size === 0) {
            fileItem.size = event.totalBytes
        }

        // 更新整体进度
        updateTaskItemProgress(taskItem, event.speed)
    }

    /**
     * 处理 Web 上传文件完成事件：标记对应文件条目为完成或失败
     */
    function handleWebUploadFileComplete(event: WebUploadFileCompleteEvent) {
        const taskItem = receiveTaskItems.value.get(`web-${event.requestId}`)
        if (!taskItem) return

        const fileItem = findFileByRecord(
            taskItem,
            event.recordId,
            event.fileName
        )
        if (fileItem) {
            fileItem.status =
                event.status === 'completed' ? 'completed' : 'failed'
            fileItem.transferredBytes = event.totalBytes
            if (event.totalBytes > 0) {
                fileItem.size = event.totalBytes
            }
            fileItem.progress =
                event.status === 'completed' ? 100 : fileItem.progress
            fileItem.speed = 0
        }

        // 更新整体进度
        updateTaskItemProgress(taskItem, 0)

        // 检查是否所有文件都已完成
        const allCompleted =
            taskItem.files.length > 0 &&
            taskItem.files.every(
                (file) =>
                    file.status === 'completed' || file.status === 'failed'
            )
        if (allCompleted) {
            const hasFailure = taskItem.files.some(
                (file) => file.status === 'failed'
            )
            taskItem.transferStatus = hasFailure ? 'failed' : 'completed'
            taskItem.completedAt = Date.now()
            taskItem.speed = 0
        }
    }

    /**
     * 通过 recordId 和文件名查找 ReceiveTaskItem 中的文件条目。
     * 由于 ReceiveTaskFileItem 没有 recordId 字段，
     * 使用 recordId → 文件名映射缓存来精确匹配。
     */
    const recordIdToFileIndex = new Map<
        string,
        { taskId: string; index: number }
    >()

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

    /**
     * 更新 ReceiveTaskItem 的整体进度信息
     */
    function updateTaskItemProgress(
        taskItem: ReceiveTaskItem,
        currentSpeed: number
    ): void {
        const totalTransferred = taskItem.files.reduce(
            (sum, file) => sum + file.transferredBytes,
            0
        )
        taskItem.totalTransferredBytes = totalTransferred
        taskItem.totalSize = taskItem.files.reduce(
            (sum, file) => sum + file.size,
            0
        )
        taskItem.progress =
            taskItem.totalSize > 0
                ? Math.round((totalTransferred / taskItem.totalSize) * 100)
                : 0
        taskItem.speed = currentSpeed
        taskItem.transferStatus = 'transferring'
    }

    /**
     * 同意统一接收任务
     */
    async function acceptReceiveTask(taskId: string): Promise<void> {
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
    async function rejectReceiveTask(taskId: string): Promise<void> {
        const taskItem = receiveTaskItems.value.get(taskId)
        if (taskItem?.source === 'webUpload' && taskItem.originalRequestId) {
            await rejectWebUploadRequest(taskItem.originalRequestId)
            return
        }
        // P2P 任务：取消传输
        if (taskItem?.originalTaskId) {
            await cancel(taskItem.originalTaskId)
        }
    }

    /**
     * 清理已完成的统一接收任务
     */
    function cleanupReceiveTaskItems(): void {
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
    }

    /**
     * 销毁 store，清理事件监听器
     */
    function destroy(): void {
        unlistenFns.forEach((unlisten) => unlisten())
        unlistenFns.length = 0
        tasks.value.clear()
        receiveTaskItems.value.clear()
        sendTaskItems.value.clear()
        webDownloadEnabled.value = false
        initialized.value = false
        // 重置页面状态
        transferMode.value = 'local'
        selectedPeerId.value = ''
        receiveMode.value = 'local'
    }

    // ============ 历史记录方法 ============

    /**
     * 检查 Tauri Store 是否可用
     */
    async function isTauriStoreAvailable(): Promise<boolean> {
        try {
            return typeof window !== 'undefined' && '__TAURI__' in window
        } catch {
            return false
        }
    }

    /**
     * 加载历史记录
     */
    async function loadHistory(): Promise<void> {
        try {
            let storage: TransferHistoryStorage | null = null

            if (await isTauriStoreAvailable()) {
                storage = await loadHistoryFromTauriStore()
            }

            if (!storage) {
                storage = loadHistoryFromLocalStorage()
            }

            if (storage) {
                // 版本迁移
                if (storage.version !== HISTORY_STORAGE_VERSION) {
                    storage = migrateHistoryStorage(storage)
                }
                historyItems.value = storage.items
            }

            historyLoaded.value = true
        } catch (error) {
            console.error('[Transfer] 加载历史记录失败:', error)
            historyItems.value = []
            historyLoaded.value = true
        }
    }

    /**
     * 保存历史记录
     */
    async function saveHistory(): Promise<boolean> {
        try {
            const storageData: TransferHistoryStorage = {
                version: HISTORY_STORAGE_VERSION,
                items: historyItems.value,
            }

            if (await isTauriStoreAvailable()) {
                await saveHistoryToTauriStore(storageData)
            } else {
                saveHistoryToLocalStorage(storageData)
            }
            return true
        } catch (error) {
            console.error('[Transfer] 保存历史记录失败:', error)
            return false
        }
    }

    /**
     * 从 localStorage 加载历史记录（降级方案）
     */
    function loadHistoryFromLocalStorage(): TransferHistoryStorage | null {
        const data = localStorage.getItem(HISTORY_STORAGE_KEY)
        if (!data) return null
        try {
            const parsed = JSON.parse(data)
            // 兼容旧版格式（直接是数组）
            if (Array.isArray(parsed)) {
                return { version: 0, items: parsed }
            }
            return parsed as TransferHistoryStorage
        } catch {
            console.warn('[Transfer] localStorage 历史记录格式无效')
            return null
        }
    }

    /**
     * 保存历史记录到 localStorage（降级方案）
     */
    function saveHistoryToLocalStorage(data: TransferHistoryStorage): void {
        localStorage.setItem(HISTORY_STORAGE_KEY, JSON.stringify(data))
    }

    /**
     * 从 Tauri Store 加载历史记录
     */
    async function loadHistoryFromTauriStore(): Promise<TransferHistoryStorage | null> {
        try {
            const { Store } = await import('@tauri-apps/plugin-store')
            const store = await Store.load('transfer-history.json')
            const history = await store.get<TransferHistoryStorage>('history')
            return history ?? null
        } catch (error) {
            console.warn('[Transfer] Tauri Store 加载历史记录失败:', error)
            return null
        }
    }

    /**
     * 保存历史记录到 Tauri Store
     */
    async function saveHistoryToTauriStore(
        data: TransferHistoryStorage
    ): Promise<void> {
        try {
            const { Store } = await import('@tauri-apps/plugin-store')
            const store = await Store.load('transfer-history.json')
            await store.set('history', data)
            await store.save()
        } catch (error) {
            console.error('[Transfer] Tauri Store 保存历史记录失败:', error)
            throw error
        }
    }

    /**
     * 历史记录存储迁移
     */
    function migrateHistoryStorage(
        oldStorage: TransferHistoryStorage
    ): TransferHistoryStorage {
        console.log(
            `[Transfer] 迁移历史记录从版本 ${oldStorage.version} 到 ${HISTORY_STORAGE_VERSION}`
        )
        return {
            ...oldStorage,
            version: HISTORY_STORAGE_VERSION,
        }
    }

    /**
     * 添加历史记录
     */
    async function addHistoryItem(task: TransferTask): Promise<void> {
        // 检查是否已存在
        if (historyItems.value.some((h) => h.id === task.id)) {
            return
        }

        const item: TransferHistoryItem = {
            id: task.id,
            fileName: task.file.name,
            fileSize: task.file.size,
            peerName: task.peer?.name || '',
            status: task.status,
            direction: task.direction,
            completedAt: task.completedAt || Date.now(),
            mode: task.mode,
            error: task.error,
        }

        historyItems.value.unshift(item)

        // 超出上限时移除最旧的记录
        if (historyItems.value.length > DEFAULT_MAX_HISTORY_COUNT) {
            historyItems.value = historyItems.value.slice(
                0,
                DEFAULT_MAX_HISTORY_COUNT
            )
        }

        await saveHistory()
    }

    /**
     * 删除单条历史记录
     */
    async function removeHistoryItem(id: string): Promise<void> {
        const index = historyItems.value.findIndex((item) => item.id === id)
        if (index !== -1) {
            historyItems.value.splice(index, 1)
            await saveHistory()
        }
    }

    /**
     * 批量删除历史记录
     */
    async function removeHistoryItems(ids: string[]): Promise<void> {
        const idSet = new Set(ids)
        historyItems.value = historyItems.value.filter(
            (item) => !idSet.has(item.id)
        )
        await saveHistory()
    }

    /**
     * 清空全部历史记录
     */
    async function clearHistory(): Promise<void> {
        historyItems.value = []
        await saveHistory()
    }

    /**
     * 切换历史记录选中状态
     */
    function toggleHistorySelection(id: string): void {
        const item = historyItems.value.find((h) => h.id === id)
        if (item) {
            item.selected = !item.selected
        }
    }

    /**
     * 全选/取消全选历史记录
     */
    function toggleAllHistorySelection(selected: boolean): void {
        historyItems.value.forEach((item) => {
            item.selected = selected
        })
    }

    /**
     * 设置历史记录筛选条件
     */
    function setHistoryFilter(filter: Partial<HistoryFilter>): void {
        historyFilter.value = { ...historyFilter.value, ...filter }
    }

    /**
     * 设置历史记录排序选项
     */
    function setHistorySort(sort: HistorySortOption): void {
        historySort.value = sort
    }

    /**
     * 执行自动清理
     */
    async function applyAutoCleanup(
        strategy: 'byTime' | 'byCount' | 'disabled',
        options?: { retentionDays?: number; maxCount?: number }
    ): Promise<void> {
        if (strategy === 'disabled') return

        let removed = false

        if (strategy === 'byTime' && options?.retentionDays) {
            const cutoff =
                Date.now() - options.retentionDays * 24 * 60 * 60 * 1000
            const before = historyItems.value.length
            historyItems.value = historyItems.value.filter(
                (item) => item.completedAt >= cutoff
            )
            removed = historyItems.value.length < before
        } else if (strategy === 'byCount' && options?.maxCount) {
            if (historyItems.value.length > options.maxCount) {
                historyItems.value = historyItems.value.slice(
                    0,
                    options.maxCount
                )
                removed = true
            }
        }

        if (removed) {
            await saveHistory()
        }
    }

    return {
        // 状态
        initialized,
        listenPort,
        receivePort,
        networkAddress,
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
        // 统一接收任务
        receiveTaskItems,
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
        // 计算属性
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
        setTransferMode,
        setSelectedPeerId,
        setReceiveMode,
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
