/**
 * 云盘相关类型定义
 */

/** 云盘类型 */
export type CloudType = 'webDAV' | 'aliyunOSS' | 'aliyunDrive'

/** 云盘账号连接状态 */
export type CloudAccountStatus = 'connected' | 'disconnected' | 'invalid'

/** 云盘账号信息 */
export interface CloudAccount {
    /** 唯一标识 */
    id: string
    /** 账号名称（用户自定义） */
    name: string
    /** 云盘类型 */
    cloudType: CloudType
    /** 默认上传/下载目录 */
    defaultDirectory: string
    /** 连接状态 */
    status: CloudAccountStatus
    /** 创建时间（Unix 毫秒时间戳） */
    createdAt: number
    /** 更新时间（Unix 毫秒时间戳） */
    updatedAt: number
}

/** 添加/更新云盘账号的输入 */
export interface CloudAccountInput {
    /** 账号名称 */
    name: string
    /** 云盘类型 */
    cloudType: CloudType
    /** 凭证信息（根据 cloudType 选择对应的凭证类型） */
    credentials:
        | WebDAVCredentials
        | AliyunOSSCredentials
        | AliyunDriveCredentials
    /** 默认目录 */
    defaultDirectory: string
    /** 初始状态（添加账号时如果测试连接通过可设置为 connected） */
    initialStatus?: CloudAccountStatus
}

/** 连接测试输入 */
export interface CloudConnectionTestInput {
    /** 云盘类型 */
    cloudType: CloudType
    /** 凭证信息 */
    credentials:
        | WebDAVCredentials
        | AliyunOSSCredentials
        | AliyunDriveCredentials
}

/** WebDAV 凭证 */
export interface WebDAVCredentials {
    type: 'webDAV'
    /** 服务器地址（如 https://dav.jianguoyun.com/dav/） */
    serverUrl: string
    /** 用户名 */
    username: string
    /** 密码/应用密码 */
    password: string
}

/** 阿里云 OSS 凭证 */
export interface AliyunOSSCredentials {
    type: 'aliyunOSS'
    /** Bucket 名称 */
    bucket: string
    /** Region ID（如 oss-cn-hangzhou） */
    region: string
    /** AccessKey ID */
    accessKeyId: string
    /** AccessKey Secret */
    accessKeySecret: string
    /** 自定义域名（可选） */
    customDomain?: string
}

/** 阿里云盘凭证 */
export interface AliyunDriveCredentials {
    type: 'aliyunDrive'
    /** Refresh Token */
    refreshToken: string
}

/** 云盘账号凭证（用于编辑时获取现有账号信息） */
export interface CloudAccountCredentials {
    /** WebDAV 凭证 */
    serverUrl?: string
    username?: string
    password?: string
    /** 阿里云 OSS 凭证 */
    bucket?: string
    region?: string
    accessKeyId?: string
    accessKeySecret?: string
    customDomain?: string
    /** 阿里云盘凭证 */
    refreshToken?: string
    /** 刷新 token 提示（部分显示） */
    refreshTokenHint?: string
}

/** 云盘文件/目录项 */
export interface CloudFileItem {
    /** 文件/目录名称 */
    name: string
    /** 完整路径 */
    path: string
    /** 是否为目录 */
    isDirectory: boolean
    /** 文件大小（目录为 undefined） */
    size?: number
    /** 最后修改时间（Unix 毫秒时间戳） */
    modified?: number
}

/** 云盘上传进度事件 */
export interface CloudUploadProgress {
    /** 账号 ID */
    accountId: string
    /** 本地文件路径 */
    localPath: string
    /** 远程文件路径 */
    remotePath: string
    /** 已上传字节数 */
    uploadedBytes: number
    /** 总字节数 */
    totalBytes: number
    /** 进度百分比（0-100） */
    progress: number
    /** 状态 */
    status: 'uploading' | 'completed' | 'failed'
}

/** 云盘下载进度事件 */
export interface CloudDownloadProgress {
    /** 账号 ID */
    accountId: string
    /** 远程文件路径 */
    remotePath: string
    /** 本地保存路径 */
    localPath: string
    /** 已下载字节数 */
    downloadedBytes: number
    /** 总字节数 */
    totalBytes: number
    /** 进度百分比（0-100） */
    progress: number
    /** 状态 */
    status: 'downloading' | 'completed' | 'failed'
}
