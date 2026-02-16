/**
 * 设备（Peer）相关类型定义
 */

/** 设备类型 */
export type DeviceType = 'desktop' | 'mobile' | 'web' | 'unknown'

/** 设备状态 */
export type PeerStatus = 'available' | 'busy' | 'offline'

/** 设备事件类型 */
export type PeerEventType = 'discovered' | 'updated' | 'offline'

/** 设备信息 */
export interface PeerInfo {
  /** 设备 ID */
  id: string
  /** 设备名称 */
  name: string
  /** IP 地址 */
  ip: string
  /** 端口号 */
  port: number
  /** 设备类型 */
  deviceType: DeviceType
  /** 发现时间戳 */
  discoveredAt: number
  /** 最后活跃时间戳 */
  lastSeen: number
  /** 设备状态 */
  status: PeerStatus
}

/** 设备发现事件 */
export interface PeerDiscoveryEvent {
  /** 事件类型 */
  eventType: PeerEventType
  /** 设备信息 */
  peer: PeerInfo
}

/** 获取设备类型显示文本 */
export function getDeviceTypeText(type: DeviceType): string {
  const typeTexts: Record<DeviceType, string> = {
    desktop: '桌面端',
    mobile: '移动端',
    web: '网页端',
    unknown: '未知',
  }
  return typeTexts[type]
}

/** 获取设备状态显示文本 */
export function getPeerStatusText(status: PeerStatus): string {
  const statusTexts: Record<PeerStatus, string> = {
    available: '在线',
    busy: '忙碌中',
    offline: '离线',
  }
  return statusTexts[status]
}

/** 获取设备状态颜色 */
export function getPeerStatusColor(status: PeerStatus): string {
  const statusColors: Record<PeerStatus, string> = {
    available: 'success',
    busy: 'warning',
    offline: 'grey',
  }
  return statusColors[status]
}

/** 获取设备显示名称 */
export function getPeerDisplayName(peer: PeerInfo): string {
  return peer.name || '未知设备'
}

/** 获取设备地址字符串 */
export function getPeerAddress(peer: PeerInfo): string {
  return `${peer.ip}:${peer.port}`
}

/** 检查设备是否在线（5秒内有活动） */
export function isPeerOnline(peer: PeerInfo): boolean {
  const now = Date.now()
  return now - peer.lastSeen < 5000
}
