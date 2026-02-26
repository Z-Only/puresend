<!-- 接收模式选择器组件 -->
<template>
    <v-card class="receive-mode-selector">
        <v-card-title class="text-subtitle-1 pb-0 d-flex align-center">
            {{ t('receiveMode.label') }}
            <!-- 本地网络模式下显示网络信息图标 -->
            <v-tooltip
                v-if="receiveMode === 'local'"
                location="top"
                open-on-click
                open-on-hover
                :content-class="
                    isDarkTheme
                        ? 'network-tooltip-dark'
                        : 'network-tooltip-light'
                "
            >
                <template #activator="{ props: tooltipProps }">
                    <v-icon
                        v-bind="tooltipProps"
                        :icon="mdiInformationOutline"
                        size="18"
                        color="primary"
                        class="ml-2"
                    />
                </template>
                <div class="network-info-tooltip">
                    <div class="network-info-item">
                        <span class="label">{{ t('network.deviceName') }}</span>
                        <span class="value">{{ networkInfo.deviceName }}</span>
                    </div>
                    <div
                        v-for="(ipAddress, index) in networkInfo.ipAddresses"
                        :key="index"
                        class="network-info-item"
                    >
                        <span class="label">{{
                            index === 0 ? t('network.ipAddress') : ''
                        }}</span>
                        <span class="value">{{ ipAddress }}</span>
                    </div>
                    <div class="network-info-item">
                        <span class="label">{{ t('network.port') }}</span>
                        <span class="value">{{ networkInfo.port }}</span>
                    </div>
                </div>
            </v-tooltip>
        </v-card-title>
        <v-card-text>
            <v-row>
                <v-col v-for="mode in modes" :key="mode.value" cols="6">
                    <v-tooltip :disabled="!mode.disabled" location="top">
                        <template #activator="{ props: tooltipProps }">
                            <v-card
                                v-bind="tooltipProps"
                                :color="
                                    receiveMode === mode.value && !mode.disabled
                                        ? 'primary'
                                        : undefined
                                "
                                :variant="
                                    receiveMode === mode.value && !mode.disabled
                                        ? 'flat'
                                        : 'outlined'
                                "
                                :class="[
                                    'mode-card',
                                    { 'mode-card-disabled': mode.disabled },
                                ]"
                                @click="handleModeChange(mode)"
                            >
                                <v-card-text
                                    class="d-flex flex-column align-center pa-4"
                                >
                                    <v-icon
                                        :icon="mode.icon"
                                        size="48"
                                        :color="
                                            receiveMode === mode.value &&
                                            !mode.disabled
                                                ? 'white'
                                                : mode.disabled
                                                  ? 'grey'
                                                  : 'primary'
                                        "
                                        class="mb-2"
                                    />
                                    <div
                                        class="text-subtitle-1 font-weight-medium"
                                        :class="{ 'text-grey': mode.disabled }"
                                    >
                                        {{ mode.label }}
                                    </div>
                                    <div
                                        class="text-body-2 text-center mt-1"
                                        :class="
                                            receiveMode === mode.value &&
                                            !mode.disabled
                                                ? 'text-white'
                                                : 'text-grey'
                                        "
                                    >
                                        {{ mode.description }}
                                    </div>
                                </v-card-text>
                            </v-card>
                        </template>
                        <span>{{ t('receiveMode.cloud.comingSoon') }}</span>
                    </v-tooltip>
                </v-col>
            </v-row>
        </v-card-text>
    </v-card>
</template>

<script setup lang="ts">
/**
 * 接收模式选择器组件
 *
 * 切换本地网络和云盘中转两种接收模式
 * 采用卡片式布局，与发送页面的传输模式选择器保持一致
 * 本地网络模式下显示网络信息提示
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTheme } from 'vuetify'
import { useTransferStore } from '@/stores/transfer'
import { useSettingsStore } from '@/stores/settings'
import { mdiWifi, mdiCloudUpload, mdiInformationOutline } from '@mdi/js'

const emit = defineEmits<{
    (e: 'change', mode: 'local' | 'cloud'): void
}>()

const { t } = useI18n()
const vuetifyTheme = useTheme()
const transferStore = useTransferStore()
const settingsStore = useSettingsStore()

const receiveMode = computed(() => transferStore.receiveMode)

// 当前是否为深色主题
const isDarkTheme = computed(() => vuetifyTheme.global.current.value.dark)

// 网络信息计算属性
const networkInfo = computed(() => ({
    deviceName: settingsStore.deviceName || '--',
    ipAddresses: transferStore.networkAddresses?.length
        ? transferStore.networkAddresses
        : ['--'],
    port:
        transferStore.receivePort > 0
            ? transferStore.receivePort.toString()
            : '--',
}))

// 模式选项
interface ModeOption {
    value: 'local' | 'cloud'
    label: string
    icon: string
    description: string
    disabled: boolean
}

const modes: ModeOption[] = [
    {
        value: 'local',
        label: t('receiveMode.local.label'),
        icon: mdiWifi,
        description: t('receiveMode.local.description'),
        disabled: false,
    },
    {
        value: 'cloud',
        label: t('receiveMode.cloud.label'),
        icon: mdiCloudUpload,
        description: t('receiveMode.cloud.description'),
        disabled: true,
    },
]

// 切换模式
function handleModeChange(mode: ModeOption) {
    // 云盘中转模式禁用，不切换
    if (mode.disabled) {
        return
    }
    transferStore.setReceiveMode(mode.value)
    emit('change', mode.value)
}
</script>

<style scoped>
.mode-card {
    cursor: pointer;
    transition: all 0.2s ease;
}

.mode-card:hover {
    transform: translateY(-2px);
}

.mode-card-disabled {
    cursor: not-allowed;
    opacity: 0.6;
}

.mode-card-disabled:hover {
    transform: none;
}

.network-info-tooltip {
    padding: 4px 0;
}

.network-info-item {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    padding: 4px 0;
}

.network-info-item .label {
    opacity: 0.7;
}

.network-info-item .value {
    font-weight: 500;
}
</style>

<style>
/* 全局样式：tooltip 主题适配 */
/* 浅色模式：浅色背景 + 深色文字 */
.network-tooltip-light {
    background: rgb(var(--v-theme-surface)) !important;
    color: rgb(var(--v-theme-on-surface)) !important;
    border: 1px solid rgba(0, 0, 0, 0.12);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

.network-tooltip-light .network-info-item .label {
    opacity: 0.7;
}

/* 深色模式：深色背景 + 浅色文字 */
.network-tooltip-dark {
    background: rgba(30, 30, 30, 0.95) !important;
    color: #fff !important;
    border: 1px solid rgba(255, 255, 255, 0.12);
}

.network-tooltip-dark .network-info-item .label {
    color: rgba(255, 255, 255, 0.7) !important;
}
</style>
