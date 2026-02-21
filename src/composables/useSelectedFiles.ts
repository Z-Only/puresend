/**
 * 已选文件管理 Composable
 * 提供文件选择、移除、统计等功能
 *
 * 注意：此 composable 使用 shareStore.selectedFiles 作为状态源，
 * 以支持 Tab 切换时保留已选文件状态
 */

import { computed } from 'vue'
import type {
    SelectedFileItem,
    SelectedFilesStats,
    FileSourceType,
    ThumbnailInfo,
} from '../types/content'
import { FILE_COUNT_LIMIT } from '../types/content'
import { formatFileSize, inferMimeType } from '../types/file'
import { useShareStore } from '../stores'

/** 判断是否为媒体文件 */
function isMediaFile(mimeType: string): boolean {
    return (
        mimeType.startsWith('image/') ||
        mimeType.startsWith('video/') ||
        mimeType.startsWith('audio/')
    )
}

/** 生成唯一 ID */
function generateId(path: string): string {
    return path
}

/**
 * 已选文件管理 Composable
 * 使用 shareStore.selectedFiles 作为状态源，支持 Tab 切换时保留状态
 */
export function useSelectedFiles() {
    const shareStore = useShareStore()

    /** 文件数量 */
    const count = computed(() => shareStore.selectedFiles.length)

    /** 总大小 */
    const totalSize = computed(() =>
        shareStore.selectedFiles.reduce((sum, file) => sum + file.size, 0)
    )

    /** 格式化的总大小 */
    const formattedSize = computed(() => formatFileSize(totalSize.value))

    /** 媒体文件数量 */
    const mediaCount = computed(
        () => shareStore.selectedFiles.filter((f) => f.isMedia).length
    )

    /** 是否达到上限 */
    const isAtLimit = computed(
        () => shareStore.selectedFiles.length >= FILE_COUNT_LIMIT
    )

    /** 剩余可选数量 */
    const remainingQuota = computed(() =>
        Math.max(0, FILE_COUNT_LIMIT - shareStore.selectedFiles.length)
    )

    /** 统计信息 */
    const stats = computed<SelectedFilesStats>(() => ({
        count: count.value,
        totalSize: totalSize.value,
        formattedSize: formattedSize.value,
        mediaCount: mediaCount.value,
        isAtLimit: isAtLimit.value,
    }))

    /** 文件路径集合（用于去重） */
    const filePathSet = computed(
        () => new Set(shareStore.selectedFiles.map((f) => f.path))
    )

    /**
     * 检查文件是否已存在
     */
    function hasFile(path: string): boolean {
        return filePathSet.value.has(path)
    }

    /**
     * 添加单个文件
     * @returns 添加结果：'added' | 'duplicate' | 'limit_exceeded'
     */
    function addFile(file: {
        path: string
        name: string
        size: number
        mimeType?: string
        sourceType: FileSourceType
        relativePath?: string
        isTemp?: boolean
        metadata?: Record<string, unknown>
    }): 'added' | 'duplicate' | 'limit_exceeded' {
        // 检查是否已存在
        if (hasFile(file.path)) {
            return 'duplicate'
        }

        // 检查是否达到上限
        if (isAtLimit.value) {
            return 'limit_exceeded'
        }

        const mimeType = file.mimeType || inferMimeType(file.name)
        const isMedia = isMediaFile(mimeType)

        const newItem: SelectedFileItem = {
            id: generateId(file.path),
            path: file.path,
            name: file.name,
            size: file.size,
            mimeType,
            sourceType: file.sourceType,
            isMedia,
            relativePath: file.relativePath,
            isTemp: file.isTemp,
            createdAt: Date.now(),
            metadata: file.metadata,
        }

        shareStore.selectedFiles.push(newItem)
        return 'added'
    }

    /**
     * 批量添加文件
     * @returns 添加结果统计
     */
    function addFiles(
        fileList: Array<{
            path: string
            name: string
            size: number
            mimeType?: string
            sourceType: FileSourceType
            relativePath?: string
            isTemp?: boolean
            metadata?: Record<string, unknown>
        }>
    ): {
        added: number
        duplicates: number
        limitExceeded: number
    } {
        let added = 0
        let duplicates = 0
        let limitExceeded = 0

        for (const file of fileList) {
            // 检查是否达到上限
            if (isAtLimit.value) {
                limitExceeded++
                continue
            }

            const result = addFile(file)
            if (result === 'added') {
                added++
            } else if (result === 'duplicate') {
                duplicates++
            } else if (result === 'limit_exceeded') {
                limitExceeded++
            }
        }

        return { added, duplicates, limitExceeded }
    }

    /**
     * 移除单个文件
     */
    function removeFile(path: string): boolean {
        const index = shareStore.selectedFiles.findIndex((f) => f.path === path)
        if (index !== -1) {
            shareStore.selectedFiles.splice(index, 1)
            return true
        }
        return false
    }

    /**
     * 清空所有文件
     */
    function clearFiles(): void {
        shareStore.clearSelectedFiles()
    }

    /**
     * 更新文件缩略图
     */
    function updateThumbnail(path: string, thumbnail: ThumbnailInfo): boolean {
        const file = shareStore.selectedFiles.find((f) => f.path === path)
        if (file) {
            file.thumbnail = thumbnail
            return true
        }
        return false
    }

    /**
     * 获取所有临时文件路径
     */
    function getTempFiles(): string[] {
        return shareStore.selectedFiles
            .filter((f) => f.isTemp)
            .map((f) => f.path)
    }

    return {
        // 状态（直接使用 store 的 selectedFiles）
        files: computed(() => shareStore.selectedFiles),

        // 计算属性
        count,
        totalSize,
        formattedSize,
        mediaCount,
        isAtLimit,
        remainingQuota,
        stats,

        // 方法
        hasFile,
        addFile,
        addFiles,
        removeFile,
        clearFiles,
        updateThumbnail,
        getTempFiles,
    }
}

export type UseSelectedFiles = ReturnType<typeof useSelectedFiles>
