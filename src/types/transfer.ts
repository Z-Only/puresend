/**
 * 传输相关类型定义
 */

import type { FileMetadata } from './file'

/** 传输模式 */
export type TransferMode = 'local' | 'cloud'

/** 任务状态 */
export type TaskStatus =
    | 'pending'
    | 'transferring'
    | 'completed'
    | 'failed'
    | 'cancelled'
    | 'interrupted'

/** 传输方向 */
export type TransferDirection = 'send' | 'receive'

/** 接收请求状态 */
export type ReceiveRequestStatus =
    | 'pending'
    | 'accepted'
    | 'rejected'
    | 'expired'

/** 接收请求 */
export interface ReceiveRequest {
    /** 请求 ID */
    id: string
    /** 发送方设备名称（仅作展示） */
    peerName: string
    /** 发送方真实 IP 地址 */
    peerIp: string
    /** 文件信息 */
    file: FileMetadata
    /** 请求时间戳 */
    requestedAt: number
    /** 请求过期时间戳 */
    expiresAt: number
    /** 请求状态 */
    status: ReceiveRequestStatus
}

/** 传输任务 */
export interface TransferTask {
    /** 任务 ID */
    id: string
    /** 文件元数据 */
    file: FileMetadata
    /** 传输模式 */
    mode: TransferMode
    /** 目标设备信息 */
    peer?: {
        id: string
        name: string
        ip: string
        port: number
        deviceType: string
        discoveredAt: number
        lastSeen: number
        status: string
    }
    /** 任务状态 */
    status: TaskStatus
    /** 进度百分比 */
    progress: number
    /** 已传输字节数 */
    transferredBytes: number
    /** 传输速度（字节/秒） */
    speed: number
    /** 预估剩余时间（秒） */
    estimatedTimeRemaining?: number
    /** 创建时间戳 */
    createdAt: number
    /** 完成时间戳 */
    completedAt?: number
    /** 传输方向 */
    direction: TransferDirection
    /** 错误信息 */
    error?: string
    /** 是否可恢复（断点续传） */
    resumable?: boolean
    /** 续传偏移量（字节） */
    resumeOffset?: number
    /** 是否已从断点恢复 */
    resumed?: boolean
    /** 是否使用加密传输 */
    encrypted?: boolean
    /** 压缩率（0-100，仅在启用压缩时有效） */
    compressionRatio?: number
}

/** 传输进度事件 */
export interface TransferProgress {
    /** 任务 ID */
    taskId: string
    /** 状态 */
    status: TaskStatus
    /** 进度百分比 */
    progress: number
    /** 已传输字节数 */
    transferredBytes: number
    /** 总字节数 */
    totalBytes: number
    /** 传输速度（字节/秒） */
    speed: number
    /** 预估剩余时间（秒） */
    estimatedTimeRemaining?: number
    /** 错误信息 */
    error?: string
}

/** 获取状态显示文本 */
export function getStatusText(status: TaskStatus): string {
    const statusTexts: Record<TaskStatus, string> = {
        pending: '等待中',
        transferring: '传输中',
        completed: '已完成',
        failed: '失败',
        cancelled: '已取消',
        interrupted: '已中断',
    }
    return statusTexts[status]
}

/** 获取状态 i18n 键 */
export function getStatusKey(status: TaskStatus): string {
    return `transfer.status.${status}`
}

/** 获取状态颜色 */
export function getStatusColor(status: TaskStatus): string {
    const statusColors: Record<TaskStatus, string> = {
        pending: 'grey',
        transferring: 'primary',
        completed: 'success',
        failed: 'error',
        cancelled: 'warning',
        interrupted: 'warning',
    }
    return statusColors[status]
}

/** 格式化传输速度 */
export function formatSpeed(bytesPerSecond: number): string {
    if (bytesPerSecond === 0) return '0 B/s'

    const units = ['B/s', 'KB/s', 'MB/s', 'GB/s']
    const k = 1024
    const i = Math.floor(Math.log(bytesPerSecond) / Math.log(k))

    return `${parseFloat((bytesPerSecond / Math.pow(k, i)).toFixed(2))} ${units[i]}`
}

/** 格式化剩余时间 */
export function formatTimeRemaining(seconds?: number): string {
    if (!seconds) return '--'

    if (seconds < 60) {
        return `${seconds} 秒`
    } else if (seconds < 3600) {
        const minutes = Math.floor(seconds / 60)
        const secs = seconds % 60
        return `${minutes} 分 ${secs} 秒`
    } else {
        const hours = Math.floor(seconds / 3600)
        const minutes = Math.floor((seconds % 3600) / 60)
        return `${hours} 小时 ${minutes} 分`
    }
}

/** 计算进度百分比 */
export function calculateProgress(transferred: number, total: number): number {
    if (total === 0) return 0
    return Math.min(Math.round((transferred / total) * 100), 100)
}

// ============ 链接分享相关类型 ============

/** 分享链接信息 */
export interface ShareLinkInfo {
    /** 分享链接列表 */
    links: string[]
    /** HTTP 服务器端口 */
    port: number
    /** 分享的文件列表 */
    files: FileMetadata[]
    /** 创建时间戳（毫秒） */
    createdAt: number
    /** 是否启用 PIN 保护 */
    pinEnabled: boolean
    /** PIN 码（仅在启用时存在） */
    pin?: string
    /** 是否自动接受所有访问请求 */
    autoAccept: boolean
    /** 分享状态 */
    status: ShareStatus
}

/** 分享状态 */
export type ShareStatus = 'active' | 'stopped' | 'expired'

/** 访问请求状态 */
export type AccessRequestStatus = 'pending' | 'accepted' | 'rejected'

/** 分享传输状态 */
export type ShareTransferStatus =
    | 'idle'
    | 'transferring'
    | 'completed'
    | 'cancelled'
    | 'failed'

/** 分享传输进度信息（保留用于兼容） */
export interface ShareTransferProgress {
    /** 当前上传的文件名 */
    fileName: string
    /** 已上传字节数 */
    downloadedBytes: number
    /** 总字节数 */
    totalBytes: number
    /** 进度百分比（0-100） */
    progress: number
    /** 上传速度（字节/秒） */
    speed: number
    /** 已完成文件数 */
    completedFiles: number
    /** 总文件数 */
    totalFiles: number
    /** 传输状态 */
    status: ShareTransferStatus
    /** 开始时间（毫秒） */
    startedAt?: number
    /** 完成时间（毫秒） */
    completedAt?: number
}

/**
 * 上传记录
 *
 * 从分享者（应用）视角来看，接收者通过链接获取文件时，
 * 应用作为文件提供方，实际上是在上传文件给接收者。
 */
export interface UploadRecord {
    /** 上传记录唯一 ID */
    id: string
    /** 文件名 */
    fileName: string
    /** 已上传字节数 */
    uploadedBytes: number
    /** 总字节数 */
    totalBytes: number
    /** 进度百分比（0-100） */
    progress: number
    /** 上传速度（字节/秒） */
    speed: number
    /** 上传状态 */
    status: ShareTransferStatus
    /** 开始时间（毫秒） */
    startedAt: number
    /** 完成时间（毫秒） */
    completedAt?: number
}

// ============ 统一发送任务类型 ============

/** 发送任务来源 */
export type SendTaskSource = 'p2p' | 'webDownload'

/** 发送任务审批状态 */
export type SendTaskApprovalStatus = 'pending' | 'accepted' | 'rejected'

/** 发送任务文件条目 */
export interface SendTaskFileItem {
    /** 文件名 */
    name: string
    /** 文件大小（字节） */
    size: number
    /** 已传输字节数 */
    transferredBytes: number
    /** 进度百分比（0-100） */
    progress: number
    /** 传输速度（字节/秒） */
    speed: number
    /** 文件状态 */
    status: TaskStatus
    /** 开始传输时间戳（毫秒） */
    startedAt?: number
    /** 上传 ID（仅 Web 下载任务使用，用于关联上传记录） */
    uploadId?: string
}

/** 统一发送任务项 */
export interface SendTaskItem {
    /** 任务唯一 ID */
    id: string
    /** 任务来源 */
    source: SendTaskSource
    /** 接收方标识（P2P 为设备名，Web 下载为 IP 地址） */
    receiverLabel: string
    /** 接收方 IP 地址 */
    receiverIp: string
    /** 文件列表 */
    files: SendTaskFileItem[]
    /** 文件总数 */
    fileCount: number
    /** 文件总大小（字节） */
    totalSize: number
    /** 已传输总字节数 */
    totalTransferredBytes: number
    /** 审批状态 */
    approvalStatus: SendTaskApprovalStatus
    /** 整体传输状态 */
    transferStatus: TaskStatus
    /** 整体进度百分比（0-100） */
    progress: number
    /** 整体传输速度（字节/秒） */
    speed: number
    /** 创建时间戳（毫秒） */
    createdAt: number
    /** 完成时间戳（毫秒） */
    completedAt?: number
    /** 错误信息 */
    error?: string
    /** 原始 P2P 任务 ID（用于关联 TransferTask） */
    originalTaskId?: string
    /** 原始访问请求 ID（用于关联 AccessRequest） */
    originalRequestId?: string
}

// ============ 统一接收任务类型 ============

/** 接收任务来源 */
export type ReceiveTaskSource = 'p2p' | 'webUpload'

/** 接收任务审批状态 */
export type ReceiveTaskApprovalStatus =
    | 'pending'
    | 'accepted'
    | 'rejected'
    | 'expired'

/** 接收任务文件条目 */
export interface ReceiveTaskFileItem {
    /** 文件名 */
    name: string
    /** 文件大小（字节） */
    size: number
    /** 已传输字节数 */
    transferredBytes: number
    /** 进度百分比（0-100） */
    progress: number
    /** 传输速度（字节/秒） */
    speed: number
    /** 文件状态 */
    status: TaskStatus
    /** 开始传输时间戳（毫秒） */
    startedAt?: number
}

/** 统一接收任务项 */
export interface ReceiveTaskItem {
    /** 任务唯一 ID */
    id: string
    /** 任务来源 */
    source: ReceiveTaskSource
    /** 发送方标识（P2P 为设备名，Web 上传为 IP 地址） */
    senderLabel: string
    /** 发送方 IP 地址 */
    senderIp: string
    /** 文件列表 */
    files: ReceiveTaskFileItem[]
    /** 文件总数 */
    fileCount: number
    /** 文件总大小（字节） */
    totalSize: number
    /** 已传输总字节数 */
    totalTransferredBytes: number
    /** 审批状态 */
    approvalStatus: ReceiveTaskApprovalStatus
    /** 整体传输状态 */
    transferStatus: TaskStatus
    /** 整体进度百分比（0-100） */
    progress: number
    /** 整体传输速度（字节/秒） */
    speed: number
    /** 创建时间戳（毫秒） */
    createdAt: number
    /** 完成时间戳（毫秒） */
    completedAt?: number
    /** 错误信息 */
    error?: string
    /** 原始 P2P 任务 ID（用于关联 TransferTask） */
    originalTaskId?: string
    /** 原始 Web 上传请求 ID（用于关联 WebUploadRequest） */
    originalRequestId?: string
}

// ============ Web 上传相关类型 ============

/** Web 上传请求状态 */
export type WebUploadRequestStatus =
    | 'pending'
    | 'accepted'
    | 'rejected'
    | 'expired'

/** Web 上传文件记录（对应后端 UploadRecord） */
export interface WebUploadRecord {
    /** 记录唯一 ID */
    id: string
    /** 文件名 */
    fileName: string
    /** 已上传字节数 */
    uploadedBytes: number
    /** 总字节数 */
    totalBytes: number
    /** 进度百分比（0-100） */
    progress: number
    /** 上传速度（字节/秒） */
    speed: number
    /** 状态：transferring / completed / failed */
    status: string
    /** 开始时间戳（毫秒） */
    startedAt: number
    /** 完成时间戳（毫秒） */
    completedAt?: number
}

/** Web 上传请求（按 IP 审批的接收任务） */
export interface WebUploadRequest {
    /** 请求唯一 ID */
    id: string
    /** 上传方 IP 地址 */
    clientIp: string
    /** 请求状态 */
    status: WebUploadRequestStatus
    /** 请求时间戳（毫秒） */
    createdAt: number
    /** 上传方 User-Agent */
    userAgent?: string
    /** 该 IP 下的所有上传文件记录 */
    uploadRecords: WebUploadRecord[]
}

/** Web 上传文件开始事件 */
export interface WebUploadFileStartEvent {
    /** 请求 ID */
    requestId: string
    /** 文件记录 ID */
    recordId: string
    /** 文件名 */
    fileName: string
    /** 文件总字节数 */
    totalBytes: number
    /** 上传方 IP */
    clientIp: string
}

/** Web 上传文件进度事件 */
export interface WebUploadFileProgressEvent {
    /** 请求 ID */
    requestId: string
    /** 文件记录 ID */
    recordId: string
    /** 文件名 */
    fileName: string
    /** 已上传字节数 */
    uploadedBytes: number
    /** 总字节数 */
    totalBytes: number
    /** 进度百分比（0-100） */
    progress: number
    /** 上传速度（字节/秒） */
    speed: number
}

/** Web 上传文件完成事件 */
export interface WebUploadFileCompleteEvent {
    /** 请求 ID */
    requestId: string
    /** 文件记录 ID */
    recordId: string
    /** 文件名 */
    fileName: string
    /** 文件总字节数 */
    totalBytes: number
    /** 状态：completed / failed */
    status: string
}

/** Web 上传服务器信息 */
export interface WebUploadInfo {
    /** 是否已启动 */
    enabled: boolean
    /** 服务器端口 */
    port: number
    /** 上传链接列表 */
    urls: string[]
}

/** 访问请求 */
export interface AccessRequest {
    /** 请求 ID */
    id: string
    /** 访问者 IP 地址 */
    ip: string
    /** 请求时间戳（毫秒） */
    requestedAt: number
    /** 请求状态 */
    status: AccessRequestStatus
    /** PIN 验证失败次数 */
    pinAttempts: number
    /** 是否被锁定（连续三次失败） */
    locked: boolean
    /** 锁定解除时间（毫秒） */
    lockedUntil?: number
    /** 用户代理（浏览器/平台信息，如 "Chrome(Android)"） */
    userAgent?: string
    /** 上传记录列表 */
    uploadRecords: UploadRecord[]
}

/** 分享设置 */
export interface ShareSettings {
    /** 是否启用 PIN 保护 */
    pinEnabled: boolean
    /** PIN 码 */
    pin?: string
    /** 是否自动接受所有访问请求 */
    autoAccept: boolean
}

/** PIN 验证结果 */
export interface PinVerifyResult {
    /** 是否验证成功 */
    success: boolean
    /** 剩余尝试次数（失败时） */
    remainingAttempts?: number
    /** 是否被锁定 */
    locked: boolean
    /** 锁定解除时间（毫秒，锁定时） */
    lockedUntil?: number
}

/**
 * 上传进度
 *
 * 从分享者视角，文件被接收者获取时的传输进度。
 */
export interface UploadProgress {
    /** 上传 ID */
    uploadId: string
    /** 文件名 */
    fileName: string
    /** 进度百分比（0-100） */
    progress: number
    /** 已上传字节数 */
    uploadedBytes: number
    /** 总字节数 */
    totalBytes: number
    /** 上传速度（字节/秒） */
    speed: number
    /** 接收者 IP */
    clientIp: string
}

// ============ 传输历史相关类型 ============

/** 历史记录存储版本 */
export const HISTORY_STORAGE_VERSION = 1

/** 历史记录存储键名 */
export const HISTORY_STORAGE_KEY = 'transfer-history'

/** 历史记录默认上限 */
export const DEFAULT_MAX_HISTORY_COUNT = 1000

/** 传输历史记录项 */
export interface TransferHistoryItem {
    /** 记录唯一标识 */
    id: string
    /** 文件名 */
    fileName: string
    /** 文件大小（字节） */
    fileSize: number
    /** 对端设备名称 */
    peerName: string
    /** 传输状态 */
    status: TaskStatus
    /** 传输方向 */
    direction: TransferDirection
    /** 完成时间戳（毫秒） */
    completedAt: number
    /** 传输模式 */
    mode?: TransferMode
    /** 错误信息（失败时） */
    error?: string
    /** 是否选中（用于批量操作） */
    selected?: boolean
}

/** 历史记录筛选条件 */
export interface HistoryFilter {
    /** 传输方向筛选 */
    direction?: TransferDirection | 'all'
    /** 状态筛选 */
    status?: TaskStatus | 'all'
}

/** 历史记录排序字段 */
export type HistorySortField = 'completedAt' | 'fileName' | 'fileSize'

/** 历史记录排序顺序 */
export type HistorySortOrder = 'asc' | 'desc'

/** 历史记录排序选项 */
export interface HistorySortOption {
    field: HistorySortField
    order: HistorySortOrder
}

/** 传输历史存储格式 */
export interface TransferHistoryStorage {
    /** 存储版本号 */
    version: number
    /** 历史记录列表 */
    items: TransferHistoryItem[]
}
