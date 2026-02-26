<!-- 设备列表组件 -->
<template>
    <v-card class="peer-list">
        <v-card-title class="d-flex align-center justify-space-between">
            <span class="text-subtitle-1">{{ t('peer.nearbyDevices') }}</span>
            <v-btn
                :icon="mdiRefresh"
                variant="text"
                size="small"
                :loading="loading"
                @click="handleRefresh"
            />
        </v-card-title>

        <v-card-text class="pa-0">
            <!-- 空状态 -->
            <div
                v-if="peers.length === 0"
                class="d-flex flex-column align-center justify-center pa-8"
            >
                <v-icon
                    :icon="mdiWifiOff"
                    size="48"
                    color="grey"
                    class="mb-2"
                />
                <div class="text-body-1 text-grey">
                    {{ t('peer.noDevices') }}
                </div>
                <div class="text-body-2 text-grey mt-1">
                    {{ t('peer.noDevicesHint') }}
                </div>
            </div>

            <!-- 设备列表 -->
            <v-list v-else density="compact">
                <v-list-item
                    v-for="peer in peers"
                    :key="peer.id"
                    :active="selectedPeerId === peer.id"
                    @click="selectPeer(peer.id)"
                >
                    <template #prepend>
                        <v-avatar
                            :color="getStatusColor(peer.status)"
                            size="32"
                        >
                            <v-icon
                                :icon="getDeviceIcon(peer.deviceType)"
                                color="white"
                                size="20"
                            />
                        </v-avatar>
                    </template>

                    <v-list-item-title>{{ peer.name }}</v-list-item-title>
                    <v-list-item-subtitle>
                        {{ peer.ip }}:{{ peer.port }}
                    </v-list-item-subtitle>

                    <template #append>
                        <v-chip
                            :color="getStatusColor(peer.status)"
                            size="x-small"
                            variant="flat"
                        >
                            {{ t(getPeerStatusKey(peer.status)) }}
                        </v-chip>
                    </template>
                </v-list-item>
            </v-list>
        </v-card-text>

        <!-- 手动添加设备 -->
        <v-divider />
        <v-card-actions>
            <v-btn
                color="primary"
                variant="text"
                block
                @click="showAddDialog = true"
            >
                <v-icon :icon="mdiPlus" class="mr-1" />
                {{ t('peer.addManual') }}
            </v-btn>
        </v-card-actions>

        <!-- 手动添加对话框 -->
        <v-dialog v-model="showAddDialog" max-width="400">
            <v-card>
                <v-card-title>{{ t('peer.addDevice') }}</v-card-title>
                <v-card-text>
                    <v-text-field
                        v-model="manualIp"
                        :label="t('peer.ipAddress')"
                        placeholder="192.168.1.100"
                        :rules="[validateIp]"
                    />
                    <v-text-field
                        v-model.number="manualPort"
                        :label="t('peer.port')"
                        placeholder="5353"
                        type="number"
                    />
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn variant="text" @click="showAddDialog = false">{{
                        t('common.cancel')
                    }}</v-btn>
                    <v-btn
                        color="primary"
                        variant="flat"
                        :disabled="!manualIp"
                        @click="handleAddManual"
                    >
                        {{ t('common.add') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>
    </v-card>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { PeerInfo, PeerStatus, DeviceType } from '../../types'
import { getPeerStatusColor, getPeerStatusKey } from '../../types'
import {
    mdiRefresh,
    mdiWifiOff,
    mdiDesktopTowerMonitor,
    mdiCellphone,
    mdiWeb,
    mdiHelpCircle,
    mdiPlus,
} from '@mdi/js'

const { t } = useI18n()

defineProps<{
    peers: PeerInfo[]
    selectedPeerId: string
    loading?: boolean
}>()

const emit = defineEmits<{
    (e: 'select', peerId: string): void
    (e: 'refresh'): void
    (e: 'addManual', ip: string, port: number): void
}>()

const showAddDialog = ref(false)
const manualIp = ref('')
const manualPort = ref(5353)

function getStatusColor(status: PeerStatus): string {
    return getPeerStatusColor(status)
}

function getDeviceIcon(type: DeviceType): string {
    const icons: Record<DeviceType, string> = {
        desktop: mdiDesktopTowerMonitor,
        mobile: mdiCellphone,
        web: mdiWeb,
        unknown: mdiHelpCircle,
    }
    return icons[type]
}

function selectPeer(peerId: string) {
    emit('select', peerId)
}

function handleRefresh() {
    emit('refresh')
}

function validateIp(value: string): boolean | string {
    if (!value) return t('peer.validation.enterIp')
    const ipRegex = /^(\d{1,3}\.){3}\d{1,3}$/
    if (!ipRegex.test(value)) return t('peer.validation.invalidIp')
    return true
}

function handleAddManual() {
    if (manualIp.value && manualPort.value) {
        emit('addManual', manualIp.value, manualPort.value)
        showAddDialog.value = false
        manualIp.value = ''
        manualPort.value = 5353
    }
}
</script>

<style scoped>
.peer-list {
    max-height: 400px;
    overflow-y: auto;
}
</style>
