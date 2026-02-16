/**
 * 设备发现服务 - Tauri 命令封装
 */

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { PeerInfo, PeerDiscoveryEvent } from '../types'

/**
 * 初始化设备发现服务
 * @param deviceName 本机设备名称（可选）
 * @param listenPort 监听端口（可选）
 */
export async function initDiscovery(
  deviceName?: string,
  listenPort?: number
): Promise<void> {
  return invoke('init_discovery', { deviceName, listenPort })
}

/**
 * 停止设备发现服务
 */
export async function stopDiscovery(): Promise<void> {
  return invoke('stop_discovery')
}

/**
 * 获取已发现的设备列表
 */
export async function getPeers(): Promise<PeerInfo[]> {
  return invoke('get_peers')
}

/**
 * 获取指定设备信息
 * @param peerId 设备ID
 */
export async function getPeer(peerId: string): Promise<PeerInfo | null> {
  return invoke('get_peer', { peerId })
}

/**
 * 手动添加设备
 * @param ip 设备IP地址
 * @param port 设备端口
 */
export async function addPeerManual(ip: string, port: number): Promise<PeerInfo> {
  return invoke('add_peer_manual', { ip, port })
}

/**
 * 检查设备是否在线
 * @param peerId 设备ID
 */
export async function isPeerOnline(peerId: string): Promise<boolean> {
  return invoke('is_peer_online', { peerId })
}

/**
 * 获取在线设备数量
 */
export async function getOnlineCount(): Promise<number> {
  return invoke('get_online_count')
}

// ============ 事件监听 ============

/** 设备发现事件监听器类型 */
export type PeerDiscoveryListener = (event: PeerDiscoveryEvent) => void

/**
 * 监听设备发现事件
 * @param listener 监听器函数
 * @returns 取消监听函数
 */
export function onPeerDiscovery(listener: PeerDiscoveryListener): Promise<UnlistenFn> {
  return listen<PeerDiscoveryEvent>('peer-discovery', (event) => {
    listener(event.payload)
  })
}
