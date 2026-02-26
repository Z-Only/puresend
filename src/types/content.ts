/**
 * 内容类型定义
 */

import {
    mdiFile,
    mdiFolder,
    mdiClipboard,
    mdiTextBox,
    mdiImageMultiple,
    mdiApplication,
} from '@mdi/js'

/** 内容类型 */
export type ContentType =
    | 'file'
    | 'folder'
    | 'clipboard'
    | 'text'
    | 'media'
    | 'app'

/** 文件来源类型 */
export type FileSourceType = ContentType

/** 缩略图尺寸 */
export interface ThumbnailSize {
    /** 宽度（像素） */
    width: number
    /** 高度（像素） */
    height: number
}

/** 缩略图配置 */
export interface ThumbnailConfig {
    /** 小尺寸缩略图 */
    small: ThumbnailSize
    /** 中等尺寸缩略图 */
    medium: ThumbnailSize
    /** 大尺寸缩略图 */
    large: ThumbnailSize
}

/** 默认缩略图配置 */
export const DEFAULT_THUMBNAIL_CONFIG: ThumbnailConfig = {
    small: { width: 64, height: 64 },
    medium: { width: 128, height: 128 },
    large: { width: 256, height: 256 },
}

/** 缩略图信息 */
export interface ThumbnailInfo {
    /** 缩略图路径（base64 或文件路径） */
    path: string
    /** 尺寸 */
    size: ThumbnailSize
    /** 是否已加载 */
    loaded: boolean
    /** 加载错误信息 */
    error?: string
}

/** 已选文件项 */
export interface SelectedFileItem {
    /** 唯一标识（使用路径作为 ID，便于去重） */
    id: string
    /** 文件路径 */
    path: string
    /** 显示名称 */
    name: string
    /** 文件大小（字节） */
    size: number
    /** MIME 类型 */
    mimeType: string
    /** 来源类型 */
    sourceType: FileSourceType
    /** 是否为媒体文件 */
    isMedia: boolean
    /** 缩略图信息（媒体文件专用） */
    thumbnail?: ThumbnailInfo
    /** 相对路径（文件夹展开时保留目录结构） */
    relativePath?: string
    /** 是否为临时文件（剪贴板/文本生成） */
    isTemp?: boolean
    /** 创建时间戳 */
    createdAt: number
    /** 额外元数据 */
    metadata?: Record<string, unknown>
}

/** 已选文件列表统计信息 */
export interface SelectedFilesStats {
    /** 文件总数 */
    count: number
    /** 总大小（字节） */
    totalSize: number
    /** 格式化的总大小 */
    formattedSize: string
    /** 媒体文件数量 */
    mediaCount: number
    /** 是否达到上限 */
    isAtLimit: boolean
}

/** 文件数量上限 */
export const FILE_COUNT_LIMIT = 1000

/** 内容项 */
export interface ContentItem {
    /** 内容类型 */
    type: ContentType
    /** 内容路径或标识 */
    path: string
    /** 显示名称 */
    name: string
    /** 大小（字节） */
    size: number
    /** MIME 类型或内容类型 */
    mimeType: string
    /** 创建时间戳 */
    createdAt: number
    /** 额外元数据 */
    metadata?: Record<string, string>
}

/** 内容类型显示信息 */
export interface ContentTypeInfo {
    /** 类型标识 */
    type: ContentType
    /** 显示名称 i18n 键 */
    labelKey: string
    /** 图标 */
    icon: string
    /** 描述 i18n 键 */
    descriptionKey: string
}

/** 获取内容类型显示信息（返回 i18n 键） */
export function getContentTypeInfo(type: ContentType): ContentTypeInfo {
    const typeInfo: Record<ContentType, ContentTypeInfo> = {
        file: {
            type: 'file',
            labelKey: 'content.type.file.label',
            icon: mdiFile,
            descriptionKey: 'content.type.file.description',
        },
        folder: {
            type: 'folder',
            labelKey: 'content.type.folder.label',
            icon: mdiFolder,
            descriptionKey: 'content.type.folder.description',
        },
        clipboard: {
            type: 'clipboard',
            labelKey: 'content.type.clipboard.label',
            icon: mdiClipboard,
            descriptionKey: 'content.type.clipboard.description',
        },
        text: {
            type: 'text',
            labelKey: 'content.type.text.label',
            icon: mdiTextBox,
            descriptionKey: 'content.type.text.description',
        },
        media: {
            type: 'media',
            labelKey: 'content.type.media.label',
            icon: mdiImageMultiple,
            descriptionKey: 'content.type.media.description',
        },
        app: {
            type: 'app',
            labelKey: 'content.type.app.label',
            icon: mdiApplication,
            descriptionKey: 'content.type.app.description',
        },
    }

    return typeInfo[type]
}

/** 获取内容类型的文件扩展名过滤器名称 i18n 键 */
export function getContentFilterNameKey(type: ContentType): string | undefined {
    if (type === 'media') {
        return 'content.filter.media'
    }
    if (type === 'app') {
        return 'content.filter.app'
    }
    return undefined
}

/** 获取内容类型的文件扩展名过滤器 */
export function getContentFilters(
    type: ContentType
): Array<{ nameKey: string; extensions: string[] }> | undefined {
    if (type === 'media') {
        return [
            {
                nameKey: 'content.filter.media',
                extensions: [
                    'jpg',
                    'jpeg',
                    'png',
                    'gif',
                    'webp',
                    'bmp',
                    'svg',
                    'mp4',
                    'mov',
                    'avi',
                    'mkv',
                    'webm',
                    'mp3',
                    'wav',
                    'flac',
                    'ogg',
                ],
            },
        ]
    }
    if (type === 'app') {
        return [
            {
                nameKey: 'content.filter.app',
                extensions: ['app', 'exe', 'dmg', 'pkg', 'deb', 'rpm'],
            },
        ]
    }
    if (type === 'file') {
        return undefined // 不过滤，支持所有文件
    }
    return undefined
}
