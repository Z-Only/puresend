/**
 * 格式化工具函数（Single Source of Truth）
 */

/** 格式化文件大小 */
export function formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 B'

    const units = ['B', 'KB', 'MB', 'GB', 'TB']
    const k = 1024
    const i = Math.floor(Math.log(bytes) / Math.log(k))

    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${units[i]}`
}

/** 格式化传输速度 */
export function formatSpeed(bytesPerSecond: number): string {
    if (bytesPerSecond === 0) return '0 B/s'

    const units = ['B/s', 'KB/s', 'MB/s', 'GB/s']
    const k = 1024
    const i = Math.floor(Math.log(bytesPerSecond) / Math.log(k))

    return `${parseFloat((bytesPerSecond / Math.pow(k, i)).toFixed(2))} ${units[i]}`
}

/** 格式化时间戳为本地时间字符串 */
export function formatTime(timestamp: number): string {
    return new Date(timestamp).toLocaleTimeString()
}

/** 获取任务状态对应的颜色 */
export function getStatusColor(status: string): string {
    const colorMap: Record<string, string> = {
        pending: 'grey',
        transferring: 'primary',
        completed: 'success',
        failed: 'error',
        cancelled: 'warning',
        interrupted: 'warning',
    }
    return colorMap[status] || 'grey'
}

/** 获取文件状态对应的颜色（等同于 getStatusColor，保持向后兼容） */
export const getFileStatusColor = getStatusColor

/** 文件类型图标映射 */
import {
    mdiFile,
    mdiFileDocument,
    mdiFileImage,
    mdiFileVideo,
    mdiFileMusic,
    mdiFilePdfBox,
    mdiFileExcel,
    mdiFileWord,
    mdiFilePowerpoint,
    mdiFileCode,
    mdiFolder,
    mdiArchive,
} from '@mdi/js'

/** 文件扩展名到图标的映射 */
export function getFileTypeIcon(
    fileName: string,
    isDirectory: boolean
): string {
    if (isDirectory) return mdiFolder

    const ext = fileName.toLowerCase().split('.').pop() || ''

    // 图片
    if (
        [
            'jpg',
            'jpeg',
            'png',
            'gif',
            'bmp',
            'webp',
            'svg',
            'ico',
            'heic',
            'heif',
        ].includes(ext)
    ) {
        return mdiFileImage
    }
    // 视频
    if (
        [
            'mp4',
            'avi',
            'mkv',
            'mov',
            'wmv',
            'flv',
            'webm',
            'm4v',
            '3gp',
        ].includes(ext)
    ) {
        return mdiFileVideo
    }
    // 音频
    if (
        ['mp3', 'wav', 'flac', 'aac', 'ogg', 'wma', 'm4a', 'ape'].includes(ext)
    ) {
        return mdiFileMusic
    }
    // PDF
    if (ext === 'pdf') return mdiFilePdfBox
    // Excel
    if (['xls', 'xlsx', 'csv', 'ods'].includes(ext)) return mdiFileExcel
    // Word
    if (['doc', 'docx', 'odt', 'rtf'].includes(ext)) return mdiFileWord
    // PowerPoint
    if (['ppt', 'pptx', 'odp'].includes(ext)) return mdiFilePowerpoint
    // 压缩包
    if (['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz'].includes(ext))
        return mdiArchive
    // 代码文件
    if (
        [
            'js',
            'ts',
            'jsx',
            'tsx',
            'vue',
            'html',
            'css',
            'scss',
            'sass',
            'less',
            'json',
            'xml',
            'yaml',
            'yml',
            'md',
            'py',
            'java',
            'c',
            'cpp',
            'h',
            'hpp',
            'go',
            'rs',
            'rb',
            'php',
            'swift',
            'kt',
            'scala',
            'sh',
            'bash',
        ].includes(ext)
    ) {
        return mdiFileCode
    }
    // 文档
    if (['txt', 'log', 'cfg', 'conf', 'ini'].includes(ext))
        return mdiFileDocument

    return mdiFile
}

/** 格式化文件大小（支持文件夹显示为 "--"） */
export function formatFileSizeSafe(
    size: number | undefined,
    isDirectory: boolean
): string {
    if (isDirectory) return '--'
    if (size === undefined || size === null || isNaN(size)) return '--'
    return formatFileSize(size)
}
