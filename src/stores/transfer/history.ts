/**
 * 传输历史记录模块
 */

import { ref, computed } from 'vue'
import type {
    TransferTask,
    TransferHistoryItem,
    HistoryFilter,
    HistorySortOption,
    TransferHistoryStorage,
} from '../../types'
import {
    HISTORY_STORAGE_VERSION,
    HISTORY_STORAGE_KEY,
    DEFAULT_MAX_HISTORY_COUNT,
} from '../../types/transfer'

// ============ 状态 ============

/** 历史记录列表 */
export const historyItems = ref<TransferHistoryItem[]>([])

/** 历史记录是否已加载 */
export const historyLoaded = ref(false)

/** 历史记录筛选条件 */
export const historyFilter = ref<HistoryFilter>({
    direction: 'all',
    status: 'all',
})

/** 历史记录排序选项 */
export const historySort = ref<HistorySortOption>({
    field: 'completedAt',
    order: 'desc',
})

// ============ 计算属性 ============

/** 历史记录总数量 */
export const historyCount = computed(() => historyItems.value.length)

/** 筛选后的历史记录 */
export const filteredHistory = computed(() => {
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
    if (historyFilter.value.status && historyFilter.value.status !== 'all') {
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
export const selectedHistoryItems = computed(() =>
    historyItems.value.filter((item) => item.selected)
)

/** 选中的历史记录数量 */
export const selectedHistoryCount = computed(
    () => selectedHistoryItems.value.length
)

// ============ 方法 ============

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
 * 加载历史记录
 */
export async function loadHistory(): Promise<void> {
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
export async function saveHistory(): Promise<boolean> {
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
 * 添加历史记录
 */
export async function addHistoryItem(
    taskOrItem: TransferTask | TransferHistoryItem
): Promise<void> {
    // 如果是 TransferTask，转换为 TransferHistoryItem
    let item: TransferHistoryItem
    if ('file' in taskOrItem) {
        // TransferTask
        const task = taskOrItem as TransferTask
        item = {
            id: task.id,
            fileName: task.file.name,
            fileSize: task.file.size,
            peerName: task.peer?.name || '',
            peerIp: task.peer?.ip,
            status: task.status,
            direction: task.direction,
            completedAt: task.completedAt || Date.now(),
            mode: task.mode,
            error: task.error,
        }
    } else {
        // TransferHistoryItem
        item = taskOrItem as TransferHistoryItem
    }

    // 检查是否已存在
    if (historyItems.value.some((h) => h.id === item.id)) {
        return
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
export async function removeHistoryItem(id: string): Promise<void> {
    const index = historyItems.value.findIndex((item) => item.id === id)
    if (index !== -1) {
        historyItems.value.splice(index, 1)
        await saveHistory()
    }
}

/**
 * 批量删除历史记录
 */
export async function removeHistoryItems(ids: string[]): Promise<void> {
    const idSet = new Set(ids)
    historyItems.value = historyItems.value.filter(
        (item) => !idSet.has(item.id)
    )
    await saveHistory()
}

/**
 * 清空全部历史记录
 */
export async function clearHistory(): Promise<void> {
    historyItems.value = []
    await saveHistory()
}

/**
 * 切换历史记录选中状态
 */
export function toggleHistorySelection(id: string): void {
    const item = historyItems.value.find((h) => h.id === id)
    if (item) {
        item.selected = !item.selected
    }
}

/**
 * 全选/取消全选历史记录
 */
export function toggleAllHistorySelection(selected: boolean): void {
    historyItems.value.forEach((item) => {
        item.selected = selected
    })
}

/**
 * 设置历史记录筛选条件
 */
export function setHistoryFilter(filter: Partial<HistoryFilter>): void {
    historyFilter.value = { ...historyFilter.value, ...filter }
}

/**
 * 设置历史记录排序选项
 */
export function setHistorySort(sort: HistorySortOption): void {
    historySort.value = sort
}

/**
 * 执行自动清理
 */
export async function applyAutoCleanup(
    strategy: 'byTime' | 'byCount' | 'disabled',
    options?: { retentionDays?: number; maxCount?: number }
): Promise<void> {
    if (strategy === 'disabled') return

    let removed = false

    if (strategy === 'byTime' && options?.retentionDays) {
        const cutoff = Date.now() - options.retentionDays * 24 * 60 * 60 * 1000
        const before = historyItems.value.length
        historyItems.value = historyItems.value.filter(
            (item) => item.completedAt >= cutoff
        )
        removed = historyItems.value.length < before
    } else if (strategy === 'byCount' && options?.maxCount) {
        if (historyItems.value.length > options.maxCount) {
            historyItems.value = historyItems.value.slice(0, options.maxCount)
            removed = true
        }
    }

    if (removed) {
        await saveHistory()
    }
}
