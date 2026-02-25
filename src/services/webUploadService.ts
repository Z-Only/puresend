/**
 * Web 上传服务 - Tauri 命令封装
 */

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
    WebUploadInfo,
    WebUploadRequest,
    WebUploadFileStartEvent,
    WebUploadFileProgressEvent,
    WebUploadFileCompleteEvent,
} from '../types'

/**
 * 启动 Web 上传服务器
 * @param receiveDirectory 接收目录
 * @param autoReceive 是否自动接收
 * @param fileOverwrite 是否覆盖文件
 */
export async function startWebUpload(
    receiveDirectory: string,
    autoReceive: boolean,
    fileOverwrite: boolean
): Promise<WebUploadInfo> {
    return invoke('start_web_upload', {
        receiveDirectory,
        autoReceive,
        fileOverwrite,
    })
}

/**
 * 停止 Web 上传服务器
 */
export async function stopWebUpload(): Promise<void> {
    return invoke('stop_web_upload')
}

/**
 * 获取 Web 上传请求列表
 */
export async function getWebUploadRequests(): Promise<WebUploadRequest[]> {
    return invoke('get_web_upload_requests')
}

/**
 * 同意 Web 上传请求
 * @param requestId 请求 ID
 */
export async function acceptWebUpload(requestId: string): Promise<void> {
    return invoke('accept_web_upload', { requestId })
}

/**
 * 拒绝 Web 上传请求
 * @param requestId 请求 ID
 */
export async function rejectWebUpload(requestId: string): Promise<void> {
    return invoke('reject_web_upload', { requestId })
}

// ============ 事件监听 ============

/**
 * 监听 Web 上传任务事件（按 IP 创建的接收任务）
 * @param listener 监听器函数
 * @returns 取消监听函数
 */
export function onWebUploadTask(
    listener: (request: WebUploadRequest) => void
): Promise<UnlistenFn> {
    return listen<WebUploadRequest>('web-upload-task', (event) => {
        listener(event.payload)
    })
}

/**
 * 监听 Web 上传状态变更事件
 * @param listener 监听器函数
 * @returns 取消监听函数
 */
export function onWebUploadStatusChanged(
    listener: (request: WebUploadRequest) => void
): Promise<UnlistenFn> {
    return listen<WebUploadRequest>('web-upload-status-changed', (event) => {
        listener(event.payload)
    })
}

/**
 * 监听 Web 上传文件开始事件
 * @param listener 监听器函数
 * @returns 取消监听函数
 */
export function onWebUploadFileStart(
    listener: (event: WebUploadFileStartEvent) => void
): Promise<UnlistenFn> {
    return listen<WebUploadFileStartEvent>('web-upload-file-start', (event) => {
        listener(event.payload)
    })
}

/**
 * 监听 Web 上传文件进度事件
 * @param listener 监听器函数
 * @returns 取消监听函数
 */
export function onWebUploadFileProgress(
    listener: (event: WebUploadFileProgressEvent) => void
): Promise<UnlistenFn> {
    return listen<WebUploadFileProgressEvent>(
        'web-upload-file-progress',
        (event) => {
            listener(event.payload)
        }
    )
}

/**
 * 监听 Web 上传文件完成事件
 * @param listener 监听器函数
 * @returns 取消监听函数
 */
export function onWebUploadFileComplete(
    listener: (event: WebUploadFileCompleteEvent) => void
): Promise<UnlistenFn> {
    return listen<WebUploadFileCompleteEvent>(
        'web-upload-file-complete',
        (event) => {
            listener(event.payload)
        }
    )
}
