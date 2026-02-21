<script setup lang="ts">
/**
 * 接收模式选择器组件
 *
 * 切换本地网络和云盘中转两种接收模式
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTransferStore } from '@/stores/transfer'

const emit = defineEmits<{
    (e: 'change', mode: 'local' | 'cloud'): void
}>()

const { t } = useI18n()
const transferStore = useTransferStore()
const receiveMode = computed(() => transferStore.receiveMode)

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
            {{ modes.find((m) => m.value === receiveMode)?.description }}
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
}

.mode-hint {
    margin-top: 8px;
    font-size: 12px;
    color: var(--warning-color, #ff9800);
    text-align: center;
}
</style>
