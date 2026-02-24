/**
 * 分享链接服务
 *
 * 提供分享链接相关的前端接口
 */

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import QRCode from 'qrcode'
import type {
    ShareLinkInfo,
    AccessRequest,
    ShareSettings,
    FileMetadata,
    DownloadProgress,
} from '../types'

// 重新导出类型供外部使用
export type { AccessRequest, DownloadProgress, ShareLinkInfo, ShareSettings }

// ============ 命令调用 ============

/**
 * 开始分享
 * @param files 要分享的文件列表
 * @param settings 分享设置
 */
export async function startShareService(
    files: FileMetadata[],
    settings: ShareSettings
): Promise<ShareLinkInfo> {
    return invoke<ShareLinkInfo>('start_share', { files, settings })
}

/**
 * 停止分享
 */
export async function stopShareService(): Promise<void> {
    return invoke('stop_share')
}

/**
 * 获取分享信息
 */
export async function getShareInfo(): Promise<ShareLinkInfo | null> {
    return invoke<ShareLinkInfo | null>('get_share_info')
}

/**
 * 获取访问请求列表
 */
export async function getAccessRequests(): Promise<AccessRequest[]> {
    return invoke<AccessRequest[]>('get_access_requests')
}

/**
 * 接受访问请求
 * @param requestId 请求 ID
 */
export async function acceptAccessRequest(requestId: string): Promise<void> {
    return invoke('accept_access_request', { requestId })
}

/**
 * 拒绝访问请求
 * @param requestId 请求 ID
 */
export async function rejectAccessRequest(requestId: string): Promise<void> {
    return invoke('reject_access_request', { requestId })
}

/**
 * 更新分享设置
 * @param settings 分享设置
 */
export async function updateShareSettingsService(
    settings: ShareSettings
): Promise<void> {
    return invoke('update_share_settings', { settings })
}

// ============ 事件监听 ============

/**
 * 监听访问请求事件
 * @param callback 回调函数
 */
export async function onAccessRequest(
    callback: (request: AccessRequest) => void
): Promise<UnlistenFn> {
    return listen<AccessRequest>('access-request', (event) => {
        callback(event.payload)
    })
}

/**
 * 监听访问请求被接受事件
 * @param callback 回调函数
 */
export async function onAccessRequestAccepted(
    callback: (request: AccessRequest) => void
): Promise<UnlistenFn> {
    return listen<AccessRequest>('access-request-accepted', (event) => {
        callback(event.payload)
    })
}

/**
 * 监听访问请求被拒绝事件
 * @param callback 回调函数
 */
export async function onAccessRequestRejected(
    callback: (request: AccessRequest) => void
): Promise<UnlistenFn> {
    return listen<AccessRequest>('access-request-rejected', (event) => {
        callback(event.payload)
    })
}

/**
 * 监听下载进度事件
 * @param callback 回调函数
 */
export async function onDownloadProgress(
    callback: (progress: DownloadProgress) => void
): Promise<UnlistenFn> {
    return listen<DownloadProgress>('download-progress', (event) => {
        callback(event.payload)
    })
}
/** 下载开始事件载荷 */
export interface DownloadStartPayload {
    /** 文件名 */
    file_name: string
    /** 文件大小 */
    file_size: number
    /** 客户端 IP */
    client_ip: string
}

/** 下载完成事件载荷 */
export interface DownloadCompletePayload {
    /** 文件名 */
    file_name: string
    /** 文件大小 */
    file_size: number
    /** 客户端 IP */
    client_ip: string
}

/**
 * 监听下载开始事件
 * @param callback 回调函数
 */
export async function onDownloadStart(
    callback: (payload: DownloadStartPayload) => void
): Promise<UnlistenFn> {
    return listen<DownloadStartPayload>('download-start', (event) => {
        callback(event.payload)
    })
}

/**
 * 监听下载完成事件
 * @param callback 回调函数
 */
export async function onDownloadComplete(
    callback: (payload: DownloadCompletePayload) => void
): Promise<UnlistenFn> {
    return listen<DownloadCompletePayload>('download-complete', (event) => {
        callback(event.payload)
    })
}
/**
 * 监听分享停止事件
 * @param callback 回调函数
 */
export async function onShareStopped(
    callback: () => void
): Promise<UnlistenFn> {
    return listen('share-stopped', () => {
        callback()
    })
}

// ============ 工具函数 ============

/**
 * 生成二维码数据 URL
 * @param text 二维码内容
 */
export async function generateQRCode(text: string): Promise<string> {
    return QRCode.toDataURL(text, {
        width: 200,
        margin: 2,
        color: {
            dark: '#000000',
            light: '#ffffff',
        },
    })
}

/**
 * 复制到剪贴板
 * @param text 要复制的文本
 */
export async function copyToClipboard(text: string): Promise<void> {
    try {
        await navigator.clipboard.writeText(text)
    } catch {
        // 降级方案
        const textArea = document.createElement('textarea')
        textArea.value = text
        textArea.style.position = 'fixed'
        textArea.style.left = '-9999px'
        document.body.appendChild(textArea)
        textArea.select()
        document.execCommand('copy')
        document.body.removeChild(textArea)
    }
}

/**
 * 生成随机 PIN 码
 * @param length PIN 码长度，默认为 6
 */
export function generateRandomPin(length: number = 6): string {
    const digits = '0123456789'
    let result = ''
    for (let i = 0; i < length; i++) {
        result += digits.charAt(Math.floor(Math.random() * digits.length))
    }
    return result
}

/**
 * 格式化分享链接显示
 * @param link 分享链接
 */
export function formatShareLink(link: string): string {
    // 简化显示，隐藏路径部分
    try {
        const url = new URL(link)
        return `${url.protocol}//${url.host}`
    } catch {
        return link
    }
}
