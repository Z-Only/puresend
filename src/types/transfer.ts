/**
 * 传输相关类型定义
 */

import type { FileMetadata } from './file'
import type { PeerInfo } from './peer'

/** 传输模式 */
export type TransferMode = 'local' | 'cloud'

/** 发送模式（仅本地网络模式下可用） */
export type SendMode = 'p2p' | 'link'

/** 任务状态 */
export type TaskStatus =
    | 'pending'
    | 'transferring'
    | 'completed'
    | 'failed'
    | 'cancelled'

/** 传输方向 */
export type TransferDirection = 'send' | 'receive'

/** 传输任务 */
export interface TransferTask {
    /** 任务 ID */
    id: string
    /** 文件元数据 */
    file: FileMetadata
    /** 传输模式 */
    mode: TransferMode
    /** 发送模式 */
    sendMode?: SendMode
    /** 目标设备（P2P 模式） */
    peer?: PeerInfo
    /** 任务状态 */
    status: TaskStatus
    /** 进度百分比（0-100） */
    progress: number
    /** 已传输字节数 */
    transferredBytes: number
    /** 传输速度（字节/秒） */
    speed: number
    /** 创建时间戳（毫秒） */
    createdAt: number
    /** 完成时间戳（毫秒） */
    completedAt?: number
    /** 错误信息 */
    error?: string
    /** 传输方向 */
    direction: TransferDirection
    /** 预估剩余时间（秒） */
    estimatedTimeRemaining?: number
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
    /** 分享链接 */
    link: string
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
}

/** 访问请求状态 */
export type AccessRequestStatus = 'pending' | 'accepted' | 'rejected'

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

/** 下载进度 */
export interface DownloadProgress {
    /** 下载 ID */
    downloadId: string
    /** 文件名 */
    fileName: string
    /** 进度百分比（0-100） */
    progress: number
    /** 已下载字节数 */
    downloadedBytes: number
    /** 总字节数 */
    totalBytes: number
    /** 下载速度（字节/秒） */
    speed: number
    /** 访问者 IP */
    clientIp: string
}
