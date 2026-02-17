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
    metadata?: Record<string, any>
}

/** 内容类型显示信息 */
export interface ContentTypeInfo {
    /** 类型标识 */
    type: ContentType
    /** 显示名称 i18n 键 */
    labelKey: string
    /** 图标 */
    icon: any
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
