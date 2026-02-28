/**
 * 网络状态服务 - Tauri 事件封装
 *
 * 封装后端 `network-changed` 事件的监听，
 * 提供网络变化类型定义和事件订阅接口。
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event'

/** 网络变化类型 */
export type NetworkChangeType = 'ip_changed' | 'disconnected' | 'reconnected'

/** 网络变化事件载荷 */
export interface NetworkChangedPayload {
    /** 变化类型 */
    changeType: NetworkChangeType
    /** 新的 IP 地址列表（网络断开时为空或仅含回环地址） */
    ipAddresses: string[]
    /** 上一次的 IP 地址列表 */
    previousIpAddresses: string[]
}

/**
 * 监听网络变化事件
 * @param callback 网络变化回调函数
 * @returns 取消监听函数
 */
export function onNetworkChanged(
    callback: (payload: NetworkChangedPayload) => void
): Promise<UnlistenFn> {
    return listen<NetworkChangedPayload>('network-changed', (event) => {
        callback(event.payload)
    })
}

/**
 * 判断 IP 列表是否表示网络断开状态
 * @param ipAddresses IP 地址列表
 * @returns 是否断开
 */
export function isNetworkDisconnected(ipAddresses: string[]): boolean {
    return (
        ipAddresses.length === 0 ||
        (ipAddresses.length === 1 && ipAddresses[0] === '127.0.0.1')
    )
}

/**
 * 使用新 IP 替换链接中的旧 IP
 * @param link 原始链接
 * @param previousIps 旧 IP 列表
 * @param newIps 新 IP 列表
 * @returns 更新后的链接
 */
export function updateLinkIp(
    link: string,
    previousIps: string[],
    newIps: string[]
): string {
    if (newIps.length === 0) return link

    const newPrimaryIp = newIps[0]

    // 尝试替换链接中的旧 IP
    for (const oldIp of previousIps) {
        if (link.includes(oldIp)) {
            return link.replace(oldIp, newPrimaryIp)
        }
    }

    // 如果旧 IP 列表为空（如从断开恢复），尝试替换回环地址
    if (link.includes('127.0.0.1')) {
        return link.replace('127.0.0.1', newPrimaryIp)
    }

    return link
}
