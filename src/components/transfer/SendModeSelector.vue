<script setup lang="ts">
/**
 * 发送模式选择器组件
 *
 * 切换 P2P 直连和链接分享两种发送模式
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { storeToRefs } from 'pinia'
import { useTransferStore } from '@/stores/transfer'

const props = defineProps<{
    /** 是否有选中的文件 */
    hasSelectedFiles?: boolean
}>()

const emit = defineEmits<{
    (e: 'change', mode: 'p2p' | 'link'): void
}>()

const { t } = useI18n()
const transferStore = useTransferStore()
const { sendMode } = storeToRefs(transferStore)

// 模式选项
const modes = computed(() => [
    {
        value: 'p2p',
        label: t('sendMode.p2p.label'),
        icon: '📡',
        description: t('sendMode.p2p.description'),
        disabled: false,
    },
    {
        value: 'link',
        label: t('sendMode.link.label'),
        icon: '🔗',
        description: t('sendMode.link.description'),
        disabled: !props.hasSelectedFiles,
    },
])

// 切换模式
function handleModeChange(mode: 'p2p' | 'link') {
    const modeOption = modes.value.find((m) => m.value === mode)
    if (modeOption?.disabled) {
        return
    }
    transferStore.setSendMode(mode)
    emit('change', mode)
}
</script>

<template>
    <div class="send-mode-selector">
        <div class="mode-label">{{ t('sendMode.label') }}</div>
        <div class="mode-options">
            <button
                v-for="mode in modes"
                :key="mode.value"
                class="mode-option"
                :class="{
                    active: sendMode === mode.value,
                    disabled: mode.disabled,
                }"
                :disabled="mode.disabled"
                @click="handleModeChange(mode.value as 'p2p' | 'link')"
            >
                <span class="mode-icon">{{ mode.icon }}</span>
                <span class="mode-label-text">{{ mode.label }}</span>
            </button>
        </div>
        <div class="mode-description">
            {{ modes.find((m) => m.value === sendMode)?.description }}
        </div>
        <!-- 未选择文件时的提示 -->
        <div
            v-if="!props.hasSelectedFiles && sendMode === 'link'"
            class="mode-hint"
        >
            {{ t('sendMode.link.selectFileHint') }}
        </div>
    </div>
</template>

<style scoped>
.send-mode-selector {
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
