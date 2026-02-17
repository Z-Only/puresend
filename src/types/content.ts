/**
 * 内容类型定义
 */

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
  /** 显示名称 */
  label: string
  /** 图标名称 */
  icon: string
  /** 描述 */
  description: string
}

/** 获取内容类型显示信息 */
export function getContentTypeInfo(type: ContentType): ContentTypeInfo {
  const typeInfo: Record<ContentType, ContentTypeInfo> = {
    file: {
      type: 'file',
      label: '文件',
      icon: 'mdi-file',
      description: '选择单个文件进行传输'
    },
    folder: {
      type: 'folder',
      label: '文件夹',
      icon: 'mdi-folder',
      description: '选择整个文件夹进行传输'
    },
    clipboard: {
      type: 'clipboard',
      label: '剪贴板',
      icon: 'mdi-clipboard',
      description: '从剪贴板导入文本内容'
    },
    text: {
      type: 'text',
      label: '文本',
      icon: 'mdi-text-box',
      description: '手动输入或粘贴文本内容'
    },
    media: {
      type: 'media',
      label: '媒体',
      icon: 'mdi-image-multiple',
      description: '选择图片、视频、音频文件'
    },
    app: {
      type: 'app',
      label: '应用',
      icon: 'mdi-application',
      description: '选择已安装的应用程序'
    }
  }
  
  return typeInfo[type]
}

/** 获取内容类型的文件扩展名过滤器 */
export function getContentFilters(type: ContentType): Array<{ name: string; extensions: string[]}> | undefined {
  if (type === 'media') {
    return [{
      name: '媒体文件',
      extensions: ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg', 'mp4', 'mov', 'avi', 'mkv', 'webm', 'mp3', 'wav', 'flac', 'ogg']
    }]
  }
  if (type === 'file') {
    return undefined // 不过滤，支持所有文件
  }
  return undefined
}
