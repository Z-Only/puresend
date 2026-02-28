/**
 * 传输事件监听模块
 */

import type { Ref } from 'vue'
import type { TransferProgress } from '../../types'
import type { UnlistenFn } from '@tauri-apps/api/event'
import type { TransferTask } from '../../types'
import { useSettingsStore } from '../settings'
import {
    onTransferProgress,
    onTransferError,
    onTransferComplete,
    onTransferInterrupted,
} from '../../services/transferService'
import type { ProgressHandler } from './types'

// ============ P2P 传输事件处理 ============

/**
 * 创建 P2P 传输进度处理器
 */
export function createProgressHandler(
    tasks: Ref<Map<string, TransferTask>>
): ProgressHandler {
    return function handleProgress(progress: TransferProgress) {
        const task = tasks.value.get(progress.taskId)
        if (task) {
            task.status = progress.status
            task.progress = progress.progress
            task.transferredBytes = progress.transferredBytes
            task.speed = progress.speed
        }
    }
}

/**
 * 创建 P2P 传输错误处理器
 */
export function createErrorHandler(
    tasks: Ref<Map<string, TransferTask>>
): ProgressHandler {
    return function handleError(progress: TransferProgress) {
        const task = tasks.value.get(progress.taskId)
        if (task) {
            task.status = 'failed'
            task.error = progress.error
        }
    }
}

/**
 * 创建 P2P 传输完成处理器
 */
export function createCompleteHandler(
    tasks: Ref<Map<string, TransferTask>>,
    addHistoryItem: (taskOrItem: TransferTask) => Promise<void>
): ProgressHandler {
    return function handleComplete(progress: TransferProgress) {
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
}

/**
 * 创建 P2P 传输中断处理器
 */
export function createInterruptedHandler(
    tasks: Ref<Map<string, TransferTask>>
): ProgressHandler {
    return function handleInterrupted(progress: TransferProgress) {
        const task = tasks.value.get(progress.taskId)
        if (task) {
            task.status = 'interrupted'
            task.resumable = true
            task.resumeOffset = progress.transferredBytes
            task.error = progress.error
        }
    }
}

/**
 * 设置 P2P 传输事件监听器
 */
export async function setupP2PEventListeners(
    tasks: Ref<Map<string, TransferTask>>,
    addHistoryItem: (taskOrItem: TransferTask) => Promise<void>
): Promise<UnlistenFn[]> {
    const unlistenFns: UnlistenFn[] = []

    unlistenFns.push(
        await onTransferProgress(createProgressHandler(tasks)),
        await onTransferError(createErrorHandler(tasks)),
        await onTransferComplete(createCompleteHandler(tasks, addHistoryItem)),
        await onTransferInterrupted(createInterruptedHandler(tasks))
    )

    return unlistenFns
}
