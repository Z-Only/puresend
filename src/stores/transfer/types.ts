/**
 * Transfer Store 类型定义
 */

import type { UnlistenFn } from '@tauri-apps/api/event'
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
} from '../../types'
import type { ResumableTaskInfo } from '../../services/transferService'

// ============ 导出类型 ============

export type {
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
}

export type { ResumableTaskInfo, UnlistenFn }

// ============ Store 状态接口 ============

export interface TransferStoreState {
    initialized: boolean
    listenPort: number
    receivePort: number
    networkAddresses: string[]
    receiveDirectory: string
    tasks: Map<string, TransferTask>
    selectedTaskId: string
    loading: boolean
    error: string
    transferMode: TransferMode
    selectedPeerId: string
    receiveMode: TransferMode

    // Web 上传状态
    webUploadEnabled: boolean
    webUploadInfo: WebUploadInfo | null
    webUploadRequests: Map<string, WebUploadRequest>
    receiveTaskItems: Map<string, ReceiveTaskItem>

    // Web 下载状态
    webDownloadEnabled: boolean
    sendTaskItems: Map<string, SendTaskItem>

    // 历史记录状态
    historyItems: TransferHistoryItem[]
    historyLoaded: boolean
    historyFilter: HistoryFilter
    historySort: HistorySortOption
}

// ============ 事件处理器类型 ============

export type ProgressHandler = (progress: TransferProgress) => void
export type WebUploadTaskHandler = (request: WebUploadRequest) => void
export type WebUploadStatusChangedHandler = (request: WebUploadRequest) => void
export type WebUploadFileStartHandler = (event: WebUploadFileStartEvent) => void
export type WebUploadFileProgressHandler = (
    event: WebUploadFileProgressEvent
) => void
export type WebUploadFileCompleteHandler = (
    event: WebUploadFileCompleteEvent
) => void

// ============ 历史记录存储相关 ============

export {
    HISTORY_STORAGE_VERSION,
    HISTORY_STORAGE_KEY,
    DEFAULT_MAX_HISTORY_COUNT,
} from '../../types/transfer'
