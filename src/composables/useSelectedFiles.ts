/**
 * 已选文件管理 Composable
 * 提供文件选择、移除、统计等功能
 */

import { ref, computed, readonly } from 'vue'
import type {
    SelectedFileItem,
    SelectedFilesStats,
    FileSourceType,
    ThumbnailInfo,
} from '../types/content'
import { FILE_COUNT_LIMIT } from '../types/content'
import { formatFileSize, inferMimeType } from '../types/file'

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
 */
export function useSelectedFiles() {
    /** 已选文件列表 */
    const files = ref<SelectedFileItem[]>([])

    /** 文件数量 */
    const count = computed(() => files.value.length)

    /** 总大小 */
    const totalSize = computed(() =>
        files.value.reduce((sum, file) => sum + file.size, 0)
    )

    /** 格式化的总大小 */
    const formattedSize = computed(() => formatFileSize(totalSize.value))

    /** 媒体文件数量 */
    const mediaCount = computed(
        () => files.value.filter((f) => f.isMedia).length
    )

    /** 是否达到上限 */
    const isAtLimit = computed(() => files.value.length >= FILE_COUNT_LIMIT)

    /** 剩余可选数量 */
    const remainingQuota = computed(
        () => Math.max(0, FILE_COUNT_LIMIT - files.value.length)
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
        () => new Set(files.value.map((f) => f.path))
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

        files.value.push(newItem)
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
        const index = files.value.findIndex((f) => f.path === path)
        if (index !== -1) {
            files.value.splice(index, 1)
            return true
        }
        return false
    }

    /**
     * 清空所有文件
     */
    function clearFiles(): void {
        files.value = []
    }

    /**
     * 更新文件缩略图
     */
    function updateThumbnail(
        path: string,
        thumbnail: ThumbnailInfo
    ): boolean {
        const file = files.value.find((f) => f.path === path)
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
        return files.value
            .filter((f) => f.isTemp)
            .map((f) => f.path)
    }

    return {
        // 状态
        files: readonly(files),

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
