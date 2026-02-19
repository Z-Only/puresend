<script setup lang="ts">
/**
 * 分享设置对话框组件
 *
 * 配置自动接受等分享选项
 * PIN 配置由 PinConfigDialog 单独处理
 */
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ShareSettings } from '@/types'

const props = defineProps<{
    visible: boolean
    settings: ShareSettings
}>()

const emit = defineEmits<{
    (e: 'update:visible', value: boolean): void
    (e: 'update:settings', value: ShareSettings): void
}>()

const { t } = useI18n()

// 本地设置状态
const localSettings = ref<ShareSettings>({
    pinEnabled: false,
    pin: '',
    autoAccept: false,
})

// 监听 props 变化
watch(
    () => props.settings,
    (newSettings) => {
        localSettings.value = { ...newSettings }
    },
    { immediate: true }
)

// 关闭对话框
function handleClose() {
    emit('update:visible', false)
}

// 应用设置
function handleApply() {
    emit('update:settings', { ...localSettings.value })
    handleClose()
}
</script>

<template>
    <Teleport to="body">
        <div v-if="visible" class="dialog-overlay" @click.self="handleClose">
            <div class="dialog-content">
                <div class="dialog-header">
                    <h3>{{ t('share.settings.title') }}</h3>
                    <button class="close-btn" @click="handleClose">×</button>
                </div>

                <div class="dialog-body">
                    <!-- 自动接受开关 -->
                    <div class="setting-item">
                        <div class="setting-label">
                            <span class="label-text">{{
                                t('share.settings.autoAccept')
                            }}</span>
                            <span class="label-hint">{{
                                t('share.settings.autoAcceptHint')
                            }}</span>
                        </div>
                        <label class="toggle">
                            <input
                                type="checkbox"
                                v-model="localSettings.autoAccept"
                            />
                            <span class="toggle-slider"></span>
                        </label>
                    </div>
                </div>

                <div class="dialog-footer">
                    <button class="cancel-btn" @click="handleClose">
                        {{ t('common.cancel') }}
                    </button>
                    <button class="apply-btn" @click="handleApply">
                        {{ t('common.apply') }}
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
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
}

.dialog-content {
    background: var(--bg-primary);
    border-radius: 12px;
    width: 90%;
    max-width: 400px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
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
    padding: 20px;
}

.setting-item {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 12px 0;
    border-bottom: 1px solid var(--border-color);
}

.setting-item:last-child {
    border-bottom: none;
}

.setting-label {
    flex: 1;
}

.label-text {
    display: block;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 4px;
}

.label-hint {
    display: block;
    font-size: 12px;
    color: var(--text-secondary);
}

/* Toggle 开关 */
.toggle {
    position: relative;
    width: 48px;
    height: 24px;
    cursor: pointer;
}

.toggle input {
    opacity: 0;
    width: 0;
    height: 0;
}

.toggle-slider {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--bg-tertiary);
    border-radius: 12px;
    transition: all 0.2s;
}

.toggle-slider::before {
    content: '';
    position: absolute;
    width: 20px;
    height: 20px;
    left: 2px;
    top: 2px;
    background: white;
    border-radius: 50%;
    transition: all 0.2s;
}

.toggle input:checked + .toggle-slider {
    background: var(--primary-color);
}

.toggle input:checked + .toggle-slider::before {
    transform: translateX(24px);
}

/* 底部按钮 */
.dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-color);
}

.cancel-btn,
.apply-btn {
    padding: 10px 20px;
    border-radius: 6px;
    font-size: 14px;
    cursor: pointer;
}

.cancel-btn {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
}

.apply-btn {
    background: var(--primary-color);
    border: none;
    color: white;
}

.apply-btn:disabled {
    background: var(--bg-tertiary);
    cursor: not-allowed;
}
</style>
