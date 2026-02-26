/**
 * 传输服务 - Tauri 命令封装
 */

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { FileMetadata, TransferTask, TransferProgress } from '../types'

/**
 * 初始化传输服务
 */
export async function initTransfer(): Promise<void> {
    return invoke('init_transfer')
}

/**
 * 获取本机监听端口
 */
export async function getTransferPort(): Promise<number> {
    return invoke('get_transfer_port')
}

/**
 * 准备文件传输（计算元数据和哈希）
 * @param filePath 文件路径
 */
export async function prepareFileTransfer(
    filePath: string
): Promise<FileMetadata> {
    return invoke('prepare_file_transfer', { filePath })
}

/**
 * 获取文件元数据（不计算哈希，仅获取基本信息）
 * @param filePath 文件路径
 */
export async function getFileMetadata(filePath: string): Promise<FileMetadata> {
    return invoke('get_file_metadata', { filePath })
}

/**
 * 发送文件（同步执行，阻塞直到完成或失败）
 * @param fileMetadata 文件元数据
 * @param peerId 目标设备ID
 * @param peerIp 目标设备IP
 * @param peerPort 目标设备端口
 */
export async function sendFile(
    fileMetadata: FileMetadata,
    peerId: string,
    peerIp: string,
    peerPort: number
): Promise<string> {
    return invoke('send_file', {
        fileMetadata,
        peerId,
        peerIp,
        peerPort,
    })
}

/**
 * 发送文件（后台执行，立即返回任务ID）
 * @param fileMetadata 文件元数据
 * @param peerId 目标设备ID
 * @param peerIp 目标设备IP
 * @param peerPort 目标设备端口
 */
export async function sendFileAsync(
    fileMetadata: FileMetadata,
    peerId: string,
    peerIp: string,
    peerPort: number
): Promise<string> {
    return invoke('send_file_async', {
        fileMetadata,
        peerId,
        peerIp,
        peerPort,
    })
}

/**
 * 取消传输
 * @param taskId 任务ID
 */
export async function cancelTransfer(taskId: string): Promise<void> {
    return invoke('cancel_transfer', { taskId })
}

/**
 * 获取传输进度
 * @param taskId 任务ID
 */
export async function getTransferProgress(
    taskId: string
): Promise<TransferProgress> {
    return invoke('get_transfer_progress', { taskId })
}

/**
 * 获取所有活跃任务
 */
export async function getActiveTasks(): Promise<TransferTask[]> {
    return invoke('get_active_tasks')
}

/**
 * 验证文件完整性
 * @param filePath 文件路径
 * @param expectedHash 期望的哈希值
 */
export async function verifyFileIntegrity(
    filePath: string,
    expectedHash: string
): Promise<boolean> {
    return invoke('verify_file_integrity', { filePath, expectedHash })
}

/**
 * 清理已完成任务
 * @returns 清理的任务数量
 */
export async function cleanupCompletedTasks(): Promise<number> {
    return invoke('cleanup_completed_tasks')
}

/**
 * 获取网络信息（不启动接收服务）
 */
export async function getNetworkInfo(): Promise<{
    isReceiving: boolean
    port: number
    networkAddresses: string[]
    shareCode: string
}> {
    return invoke('get_network_info')
}

/**
 * 启动接收监听服务器
 * @param port 可选的指定端口，不传则自动分配
 */
export async function startReceiving(port?: number): Promise<{
    isReceiving: boolean
    port: number
    networkAddresses: string[]
    shareCode: string
    autoReceive: boolean
    fileOverwrite: boolean
}> {
    return invoke('start_receiving', { port })
}

/**
 * 停止接收监听服务器
 */
export async function stopReceiving(): Promise<void> {
    return invoke('stop_receiving')
}

/**
 * 获取接收目录
 */
export async function getReceiveDirectory(): Promise<string> {
    return invoke('get_receive_directory')
}

/**
 * 设置接收目录
 * @param directory 目录路径
 */
export async function setReceiveDirectory(directory: string): Promise<void> {
    return invoke('set_receive_directory', { directory })
}

// ============ 事件监听 ============

/** 传输进度事件监听器类型 */
export type TransferProgressListener = (progress: TransferProgress) => void

/** 传输错误事件监听器类型 */
export type TransferErrorListener = (progress: TransferProgress) => void

/** 传输完成事件监听器类型 */
export type TransferCompleteListener = (progress: TransferProgress) => void

/**
 * 监听传输进度事件
 * @param listener 监听器函数
 * @returns 取消监听函数
 */
export function onTransferProgress(
    listener: TransferProgressListener
): Promise<UnlistenFn> {
    return listen<TransferProgress>('transfer-progress', (event) => {
        listener(event.payload)
    })
}

/**
 * 监听传输错误事件
 * @param listener 监听器函数
 * @returns 取消监听函数
 */
export function onTransferError(
    listener: TransferErrorListener
): Promise<UnlistenFn> {
    return listen<TransferProgress>('transfer-error', (event) => {
        listener(event.payload)
    })
}

/**
 * 监听传输完成事件
 * @param listener 监听器函数
 * @returns 取消监听函数
 */
export function onTransferComplete(
    listener: TransferCompleteListener
): Promise<UnlistenFn> {
    return listen<TransferProgress>('transfer-complete', (event) => {
        listener(event.payload)
    })
}
