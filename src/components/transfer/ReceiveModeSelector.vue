<script setup lang="ts">
/**
 * 接收模式选择器组件
 *
 * 切换本地网络和云盘中转两种接收模式
 * 本地网络模式下显示网络信息提示
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTheme } from 'vuetify'
import { useTransferStore } from '@/stores/transfer'
import { useSettingsStore } from '@/stores/settings'
import { mdiInformationOutline } from '@mdi/js'

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
    ipAddress: transferStore.networkAddress || '--',
    port:
        transferStore.receivePort > 0
            ? transferStore.receivePort.toString()
            : '--',
}))

// 模式选项
const modes = computed(() => [
    {
        value: 'local',
        label: t('receiveMode.local.label'),
        icon: '📡',
        description: t('receiveMode.local.description'),
        disabled: false,
    },
    {
        value: 'cloud',
        label: t('receiveMode.cloud.label'),
        icon: '☁️',
        description: t('receiveMode.cloud.description'),
        disabled: true,
    },
])

// 切换模式
function handleModeChange(mode: 'local' | 'cloud') {
    const modeOption = modes.value.find((m) => m.value === mode)
    if (modeOption?.disabled) {
        return
    }
    transferStore.setReceiveMode(mode)
    emit('change', mode)
}
</script>

<template>
    <div class="receive-mode-selector">
        <div class="mode-label">{{ t('receiveMode.label') }}</div>
        <div class="mode-options">
            <button
                v-for="mode in modes"
                :key="mode.value"
                class="mode-option"
                :class="{
                    active: receiveMode === mode.value,
                    disabled: mode.disabled,
                }"
                :disabled="mode.disabled"
                @click="handleModeChange(mode.value as 'local' | 'cloud')"
            >
                <span class="mode-icon">{{ mode.icon }}</span>
                <span class="mode-label-text">{{ mode.label }}</span>
            </button>
        </div>
        <div class="mode-description">
            <span>{{
                modes.find((m) => m.value === receiveMode)?.description
            }}</span>
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
                        size="16"
                        color="primary"
                        class="ml-1 info-icon"
                    />
                </template>
                <div class="network-info-tooltip">
                    <div class="network-info-item">
                        <span class="label"
                            >{{ t('network.deviceName') }}:</span
                        >
                        <span class="value">{{ networkInfo.deviceName }}</span>
                    </div>
                    <div class="network-info-item">
                        <span class="label">{{ t('network.ipAddress') }}:</span>
                        <span class="value">{{ networkInfo.ipAddress }}</span>
                    </div>
                    <div class="network-info-item">
                        <span class="label">{{ t('network.port') }}:</span>
                        <span class="value">{{ networkInfo.port }}</span>
                    </div>
                </div>
            </v-tooltip>
        </div>
        <!-- 云盘模式开发中提示 -->
        <div v-if="receiveMode === 'cloud'" class="mode-hint">
            {{ t('receiveMode.cloud.comingSoon') }}
        </div>
    </div>
</template>

<style scoped>
.receive-mode-selector {
    margin-bottom: 20px;
}

.mode-label {
    font-size: 13px;
    color: var(--text-secondary);
    margin-bottom: 8px;
}

.mode-options {
    display: flex;
    gap: 8px;
}

.mode-option {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 12px 16px;
    background: var(--bg-secondary);
    border: 2px solid transparent;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s;
}

.mode-option:hover:not(.disabled) {
    background: var(--bg-tertiary);
}

.mode-option.active {
    background: var(--primary-bg);
    border-color: var(--primary-color);
}

.mode-option.disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.mode-icon {
    font-size: 20px;
}

.mode-label-text {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
}

.mode-description {
    margin-top: 8px;
    font-size: 12px;
    color: var(--text-secondary);
    text-align: center;
    display: flex;
    align-items: center;
    justify-content: center;
}

.info-icon {
    cursor: pointer;
    vertical-align: middle;
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

.mode-hint {
    margin-top: 8px;
    font-size: 12px;
    color: var(--warning-color, #ff9800);
    text-align: center;
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
