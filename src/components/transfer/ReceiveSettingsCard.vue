<!-- 接收设置卡片组件 -->
<template>
    <v-card class="receive-settings-card">
        <v-card-title class="text-subtitle-1">
            {{ t('receive.settings.title') }}
        </v-card-title>
        <v-card-text>
            <!-- 接收目录 -->
            <div class="setting-item mb-4">
                <div
                    class="setting-label text-body-2 text-medium-emphasis mb-2"
                >
                    {{ t('receive.receiveDirectory') }}
                </div>
                <v-text-field
                    :model-value="transferStore.receiveDirectory"
                    :label="t('receive.saveLocation')"
                    readonly
                    variant="outlined"
                    density="compact"
                    hide-details
                    :append-icon="mdiFolderOpen"
                    @click:append="handleSelectDirectory"
                />
            </div>

            <!-- 自动接收开关 -->
            <div class="setting-item mb-4">
                <div class="d-flex align-center justify-space-between">
                    <div>
                        <div class="setting-label text-body-1">
                            {{ t('receive.settings.autoReceive') }}
                        </div>
                        <div class="text-body-2 text-medium-emphasis">
                            {{ t('receive.settings.autoReceiveHint') }}
                        </div>
                    </div>
                    <v-switch
                        v-model="autoReceive"
                        color="primary"
                        hide-details
                        @update:model-value="handleAutoReceiveChange"
                    />
                </div>
            </div>

            <!-- 文件覆盖开关 -->
            <div class="setting-item">
                <div class="d-flex align-center justify-space-between">
                    <div>
                        <div class="setting-label text-body-1">
                            {{ t('receive.settings.fileOverwrite') }}
                        </div>
                        <div class="text-body-2 text-medium-emphasis">
                            {{ t('receive.settings.fileOverwriteHint') }}
                        </div>
                    </div>
                    <v-switch
                        v-model="fileOverwrite"
                        color="primary"
                        hide-details
                        @update:model-value="handleFileOverwriteChange"
                    />
                </div>
            </div>
        </v-card-text>
    </v-card>
</template>

<script setup lang="ts">
/**
 * 接收设置卡片组件
 *
 * 展示接收相关的设置项：
 * - 接收目录（从 transferStore 读取）
 * - 自动接收开关（从 settingsStore 读取）
 * - 文件覆盖开关（从 settingsStore 读取）
 */
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { open } from '@tauri-apps/plugin-dialog'
import { useTransferStore } from '@/stores/transfer'
import { useSettingsStore } from '@/stores/settings'
import { mdiFolderOpen } from '@mdi/js'

const { t } = useI18n()
const transferStore = useTransferStore()
const settingsStore = useSettingsStore()

// 本地状态（与 store 同步）
const autoReceive = ref(settingsStore.receiveSettings.autoReceive)
const fileOverwrite = ref(settingsStore.receiveSettings.fileOverwrite)

// 监听 store 变化，更新本地状态
watch(
    () => settingsStore.receiveSettings.autoReceive,
    (val) => {
        autoReceive.value = val
    }
)

watch(
    () => settingsStore.receiveSettings.fileOverwrite,
    (val) => {
        fileOverwrite.value = val
    }
)

// 选择接收目录
async function handleSelectDirectory() {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            title: t('receive.selectDirectory'),
        })

        if (selected && typeof selected === 'string') {
            await transferStore.updateReceiveDirectory(selected)
        }
    } catch (error) {
        console.error('[ReceiveSettingsCard] 选择目录失败:', error)
    }
}

// 处理自动接收开关变化
async function handleAutoReceiveChange(value: boolean | null) {
    if (value !== null) {
        await settingsStore.setAutoReceive(value)
    }
}

// 处理文件覆盖开关变化
async function handleFileOverwriteChange(value: boolean | null) {
    if (value !== null) {
        await settingsStore.setFileOverwrite(value)
    }
}
</script>

<style scoped>
.receive-settings-card {
    margin-bottom: 16px;
}

.setting-item {
    padding: 8px 0;
}

.setting-label {
    font-weight: 500;
}
</style>
