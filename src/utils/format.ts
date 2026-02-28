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
