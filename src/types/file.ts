/**
 * 文件相关类型定义
 */

/** 分块信息 */
export interface ChunkInfo {
    /** 块序号（从 0 开始） */
    index: number
    /** 块大小（字节） */
    size: number
    /** 块偏移量 */
    offset: number
    /** 块哈希 */
    hash: string
}

/** 文件元数据 */
export interface FileMetadata {
    /** 文件唯一标识 */
    id: string
    /** 文件名 */
    name: string
    /** 文件大小（字节） */
    size: number
    /** MIME 类型（Rust 后端返回的字段名） */
    mime_type: string
    /** 文件哈希（用于校验） */
    hash: string
    /** 分块信息 */
    chunks: ChunkInfo[]
    /** 文件路径（发送时为源路径，接收时为目标路径） */
    path?: string
}

/** 根据文件扩展名推断 MIME 类型 */
export function inferMimeType(filename: string): string {
    const extension = filename.split('.').pop()?.toLowerCase() || ''

    const mimeTypes: Record<string, string> = {
        // 文本类型
        txt: 'text/plain',
        md: 'text/markdown',
        json: 'application/json',
        xml: 'application/xml',
        html: 'text/html',
        htm: 'text/html',
        css: 'text/css',
        js: 'application/javascript',
        ts: 'application/typescript',

        // 图像类型
        jpg: 'image/jpeg',
        jpeg: 'image/jpeg',
        png: 'image/png',
        gif: 'image/gif',
        webp: 'image/webp',
        svg: 'image/svg+xml',
        bmp: 'image/bmp',
        ico: 'image/x-icon',

        // 视频类型
        mp4: 'video/mp4',
        avi: 'video/x-msvideo',
        mov: 'video/quicktime',
        mkv: 'video/x-matroska',
        webm: 'video/webm',

        // 音频类型
        mp3: 'audio/mpeg',
        wav: 'audio/wav',
        ogg: 'audio/ogg',
        flac: 'audio/flac',

        // 文档类型
        pdf: 'application/pdf',
        doc: 'application/msword',
        docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
        xls: 'application/vnd.ms-excel',
        xlsx: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
        ppt: 'application/vnd.ms-powerpoint',
        pptx: 'application/vnd.openxmlformats-officedocument.presentationml.presentation',

        // 压缩文件
        zip: 'application/zip',
        rar: 'application/vnd.rar',
        '7z': 'application/x-7z-compressed',
        tar: 'application/x-tar',
        gz: 'application/gzip',
    }

    return mimeTypes[extension] || 'application/octet-stream'
}

/** 格式化文件大小 */
export function formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 B'

    const units = ['B', 'KB', 'MB', 'GB', 'TB']
    const k = 1024
    const i = Math.floor(Math.log(bytes) / Math.log(k))

    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${units[i]}`
}

/** 获取文件类型图标名称 */
export function getFileIcon(mimeType: string): string {
    if (mimeType.startsWith('image/')) return 'image'
    if (mimeType.startsWith('video/')) return 'video'
    if (mimeType.startsWith('audio/')) return 'audio'
    if (mimeType === 'application/pdf') return 'pdf'
    if (mimeType.includes('word') || mimeType.includes('document'))
        return 'document'
    if (mimeType.includes('excel') || mimeType.includes('spreadsheet'))
        return 'spreadsheet'
    if (mimeType.includes('powerpoint') || mimeType.includes('presentation'))
        return 'presentation'
    if (
        mimeType.includes('zip') ||
        mimeType.includes('compressed') ||
        mimeType.includes('rar')
    )
        return 'archive'
    if (mimeType.startsWith('text/')) return 'text'
    return 'file'
}
