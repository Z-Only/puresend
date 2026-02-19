<script setup lang="ts">
/**
 * PIN 码配置对话框组件
 *
 * 用于配置分享链接的 PIN 码保护
 * - 默认生成随机 6 字符 PIN 值
 * - 可修改 PIN 值（不限制字符长度和类型）
 * - 确认后生效，取消则不生效
 */
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { generateRandomPin } from '@/services/shareService'

const props = defineProps<{
    visible: boolean
    currentPin?: string
    pinEnabled: boolean
}>()

const emit = defineEmits<{
    (e: 'update:visible', value: boolean): void
    (e: 'confirm', pin: string): void
}>()

const { t } = useI18n()

// 本地 PIN 输入
const pinInput = ref('')

// 初始化时生成随机 PIN
onMounted(() => {
    if (props.visible) {
        resetPin()
    }
})

// 监听对话框打开，重新生成随机 PIN
watch(
    () => props.visible,
    (newVisible) => {
        if (newVisible) {
            resetPin()
        }
    }
)

// 重置为随机 PIN
function resetPin() {
    pinInput.value = generateRandomPin()
}

// 关闭对话框（取消）
function handleClose() {
    emit('update:visible', false)
}

// 确认配置
function handleConfirm() {
    emit('confirm', pinInput.value)
    emit('update:visible', false)
}

// 是否可以确认
const canConfirm = computed(() => {
    return pinInput.value.length > 0
})
</script>

<template>
    <Teleport to="body">
        <div v-if="visible" class="dialog-overlay" @click.self="handleClose">
            <div class="dialog-content">
                <div class="dialog-header">
                    <h3>{{ t('share.pinConfig.title') }}</h3>
                    <button class="close-btn" @click="handleClose">×</button>
                </div>

                <div class="dialog-body">
                    <!-- PIN 输入区域 -->
                    <div class="pin-section">
                        <div class="setting-label">
                            <span class="label-text">{{
                                t('share.pinConfig.pinLabel')
                            }}</span>
                            <span class="label-hint">{{
                                t('share.pinConfig.pinHint')
                            }}</span>
                        </div>

                        <div class="pin-input-wrapper">
                            <input
                                type="text"
                                v-model="pinInput"
                                :placeholder="
                                    t('share.pinConfig.pinPlaceholder')
                                "
                                class="pin-input"
                                :class="{ error: !canConfirm }"
                            />
                            <button
                                class="refresh-btn"
                                @click="resetPin"
                                type="button"
                                :title="t('share.pinConfig.refresh')"
                            >
                                🔄
                            </button>
                        </div>

                        <div class="pin-tips">
                            {{ t('share.pinConfig.tips') }}
                        </div>
                    </div>
                </div>

                <div class="dialog-footer">
                    <button class="cancel-btn" @click="handleClose">
                        {{ t('common.cancel') }}
                    </button>
                    <button
                        class="confirm-btn"
                        :disabled="!canConfirm"
                        @click="handleConfirm"
                    >
                        {{ t('common.confirm') }}
                    </button>
                </div>
            </div>
        </div>
    </Teleport>
</template>

<style scoped>
.dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.25);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
}

.dialog-content {
    background-color: var(--v-theme-surface);
    border-radius: 12px;
    width: 90%;
    max-width: 400px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
    opacity: 1;
    z-index: 1001;
}

.dialog-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-color);
}

.dialog-header h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
}

.close-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    font-size: 24px;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 4px;
}

.close-btn:hover {
    background: var(--bg-secondary);
}

.dialog-body {
    padding: 24px 20px;
}

.pin-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.setting-label {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.label-text {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
}

.label-hint {
    font-size: 12px;
    color: var(--text-secondary);
}

.pin-input-wrapper {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-secondary);
    border-radius: 8px;
    padding: 8px 12px;
    border: 1px solid var(--border-color);
}

.pin-input-wrapper:focus-within {
    border-color: var(--primary-color);
}

.pin-input {
    flex: 1;
    border: none;
    background: transparent;
    font-size: 16px;
    color: var(--text-primary);
    outline: none;
    min-width: 0;
}

.pin-input.error {
    color: var(--error-color);
}

.refresh-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 18px;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.2s;
}

.refresh-btn:hover {
    background: var(--bg-secondary);
}

.pin-tips {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.5;
    padding: 8px;
    background: var(--bg-secondary);
    border-radius: 6px;
}

.dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-color);
}

.cancel-btn,
.confirm-btn {
    padding: 8px 20px;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    border: none;
}

.cancel-btn {
    background: var(--bg-secondary);
    color: var(--text-primary);
    font-weight: 500;
}

.cancel-btn:hover {
    background: var(--border-color);
}

.confirm-btn {
    background: var(--primary-color);
    color: var(--text-primary);
    font-weight: 600;
}

.confirm-btn:hover {
    opacity: 0.9;
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

.confirm-btn:disabled {
    background: var(--border-color);
    cursor: not-allowed;
    opacity: 0.6;
}
</style>
