/**
 * 设备发现状态管理
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { PeerInfo, PeerDiscoveryEvent, DeviceType } from '../types'
import {
    initDiscovery,
    stopDiscovery,
    getPeers,
    addPeerManual,
    isPeerOnline as checkPeerOnline,
    onPeerDiscovery,
} from '../services'
import type { UnlistenFn } from '@tauri-apps/api/event'

export const useDiscoveryStore = defineStore('discovery', () => {
    // ============ 状态 ============

    /** 是否已初始化 */
    const initialized = ref(false)

    /** 已发现的设备 */
    const peers = ref<Map<string, PeerInfo>>(new Map())

    /** 当前选中的设备ID */
    const selectedPeerId = ref<string>('')

    /** 是否正在扫描 */
    const scanning = ref(false)

    /** 错误信息 */
    const error = ref<string>('')

    /** 本机设备名称 */
    const deviceName = ref<string>('')

    /** 事件监听器清理函数 */
    let unlistenFns: UnlistenFn[] = []

    // ============ 计算属性 ============

    /** 设备列表 */
    const peerList = computed(() => Array.from(peers.value.values()))

    /** 当前选中的设备 */
    const selectedPeer = computed(() => {
        if (!selectedPeerId.value) return null
        return peers.value.get(selectedPeerId.value) || null
    })

    /** 在线设备 */
    const onlinePeers = computed(() =>
        peerList.value.filter((p) => p.status === 'available')
    )

    /** 按设备类型分组 */
    const peersByType = computed(() => {
        const groups: Record<DeviceType, PeerInfo[]> = {
            desktop: [],
            mobile: [],
            web: [],
            unknown: [],
        }

        for (const peer of peerList.value) {
            groups[peer.deviceType].push(peer)
        }

        return groups
    })

    /** 在线设备数量 */
    const onlineCount = computed(() => onlinePeers.value.length)

    // ============ 方法 ============

    /**
     * 初始化设备发现服务
     */
    async function initialize(name?: string, port?: number) {
        if (initialized.value) return

        scanning.value = true
        error.value = ''

        try {
            // 初始化发现服务
            await initDiscovery(name, port)

            // 设置设备名称
            deviceName.value = name || ''

            // 获取已发现的设备
            const existingPeers = await getPeers()
            peers.value = new Map(existingPeers.map((p) => [p.id, p]))

            // 注册事件监听
            unlistenFns.push(await onPeerDiscovery(handleDiscoveryEvent))

            initialized.value = true
        } catch (e) {
            error.value = `初始化失败: ${e}`
            console.error('初始化设备发现服务失败:', e)
        } finally {
            scanning.value = false
        }
    }

    /**
     * 处理设备发现事件
     */
    function handleDiscoveryEvent(event: PeerDiscoveryEvent) {
        const { eventType, peer } = event

        switch (eventType) {
            case 'discovered':
            case 'updated':
                peers.value.set(peer.id, peer)
                break
            case 'offline':
                peers.value.delete(peer.id)
                // 如果选中的设备离线，清除选择
                if (selectedPeerId.value === peer.id) {
                    selectedPeerId.value = ''
                }
                break
        }
    }

    /**
     * 刷新设备列表
     */
    async function refresh() {
        try {
            const peerList = await getPeers()
            peers.value = new Map(peerList.map((p) => [p.id, p]))
        } catch (e) {
            error.value = `刷新失败: ${e}`
            console.error('刷新设备列表失败:', e)
        }
    }

    /**
     * 手动添加设备
     * @param ip IP地址
     * @param port 端口号
     */
    async function addManual(
        ip: string,
        port: number
    ): Promise<PeerInfo | null> {
        try {
            const peer = await addPeerManual(ip, port)
            peers.value.set(peer.id, peer)
            return peer
        } catch (e) {
            error.value = `添加设备失败: ${e}`
            console.error('手动添加设备失败:', e)
            return null
        }
    }

    /**
     * 选择设备
     * @param peerId 设备ID
     */
    function selectPeer(peerId: string) {
        selectedPeerId.value = peerId
    }

    /**
     * 检查设备是否在线
     * @param peerId 设备ID
     */
    async function checkOnline(peerId: string): Promise<boolean> {
        try {
            return await checkPeerOnline(peerId)
        } catch (e) {
            console.error('检查设备状态失败:', e)
            return false
        }
    }

    /**
     * 停止发现服务
     */
    async function stop() {
        try {
            await stopDiscovery()
            initialized.value = false
        } catch (e) {
            console.error('停止发现服务失败:', e)
        }
    }

    /**
     * 清除错误
     */
    function clearError() {
        error.value = ''
    }

    /**
     * 销毁 - 清理资源
     */
    function destroy() {
        unlistenFns.forEach((fn) => fn())
        unlistenFns = []
        peers.value.clear()
        selectedPeerId.value = ''
        initialized.value = false
    }

    return {
        // 状态
        initialized,
        peers,
        selectedPeerId,
        scanning,
        error,
        deviceName,

        // 计算属性
        peerList,
        selectedPeer,
        onlinePeers,
        peersByType,
        onlineCount,

        // 方法
        initialize,
        refresh,
        addManual,
        selectPeer,
        checkOnline,
        stop,
        clearError,
        destroy,
    }
})
