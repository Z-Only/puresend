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
            <div class="setting-item mb-4">
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

            <!-- Web 上传开关 -->
            <div class="setting-item">
                <div class="d-flex align-center justify-space-between">
                    <div>
                        <div class="setting-label text-body-1">
                            {{ t('receive.settings.webUpload') }}
                        </div>
                        <div class="text-body-2 text-medium-emphasis">
                            {{ t('receive.settings.webUploadHint') }}
                        </div>
                    </div>
                    <v-switch
                        v-model="webUploadEnabled"
                        color="primary"
                        hide-details
                        :loading="webUploadLoading"
                        @update:model-value="handleWebUploadChange"
                    />
                </div>

                <!-- Web 上传链接和二维码 -->
                <div
                    v-if="
                        transferStore.webUploadEnabled &&
                        transferStore.webUploadInfo
                    "
                    class="mt-4"
                >
                    <div class="d-flex align-center mb-2">
                        <div class="text-body-2 text-medium-emphasis mr-2">
                            {{ t('receive.settings.webUploadLink') }}:
                        </div>
                        <code class="text-body-2">{{
                            transferStore.webUploadInfo.url
                        }}</code>
                        <v-btn
                            :icon="mdiContentCopy"
                            size="x-small"
                            variant="text"
                            class="ml-1"
                            @click="handleCopyLink"
                        />
                        <!-- 二维码图标，点击或悬停显示二维码 -->
                        <v-tooltip
                            location="top"
                            open-on-hover
                            :content-class="
                                isDarkTheme
                                    ? 'qr-tooltip-dark'
                                    : 'qr-tooltip-light'
                            "
                        >
                            <template #activator="{ props: tooltipProps }">
                                <v-btn
                                    v-bind="tooltipProps"
                                    :icon="mdiQrcode"
                                    size="x-small"
                                    variant="text"
                                    class="ml-1"
                                />
                            </template>
                            <div class="qr-tooltip-content">
                                <img
                                    v-if="qrCodeDataUrl"
                                    :src="qrCodeDataUrl"
                                    :alt="t('receive.settings.webUploadQrcode')"
                                    class="qr-code-tooltip-image"
                                />
                                <div v-else class="qr-code-loading">
                                    {{ t('share.qrcode.generating') }}
                                </div>
                            </div>
                        </v-tooltip>
                    </div>
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
import { ref, watch, onUnmounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTheme } from 'vuetify'
import { open } from '@tauri-apps/plugin-dialog'
import { useTransferStore } from '@/stores/transfer'
import { useSettingsStore } from '@/stores/settings'
import { mdiFolderOpen, mdiContentCopy, mdiQrcode } from '@mdi/js'
import QRCode from 'qrcode'

const { t } = useI18n()
const vuetifyTheme = useTheme()
const transferStore = useTransferStore()
const settingsStore = useSettingsStore()

// 当前是否为深色主题
const isDarkTheme = computed(() => vuetifyTheme.global.current.value.dark)

// 本地状态（与 store 同步）
const autoReceive = ref(settingsStore.receiveSettings.autoReceive)
const fileOverwrite = ref(settingsStore.receiveSettings.fileOverwrite)
const webUploadEnabled = ref(transferStore.webUploadEnabled)
const webUploadLoading = ref(false)
const qrCodeDataUrl = ref<string>('')

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

watch(
    () => transferStore.webUploadEnabled,
    (val) => {
        webUploadEnabled.value = val
    }
)

watch(
    () => transferStore.webUploadInfo,
    async (info) => {
        if (info?.url) {
            try {
                qrCodeDataUrl.value = await QRCode.toDataURL(info.url, {
                    width: 200,
                    margin: 2,
                })
            } catch (e) {
                console.error('生成二维码失败:', e)
            }
        } else {
            qrCodeDataUrl.value = ''
        }
    },
    { immediate: true }
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

// 处理 Web 上传开关变化
async function handleWebUploadChange(value: boolean | null) {
    if (value === null) return
    webUploadLoading.value = true
    try {
        if (value) {
            await transferStore.startWebUpload()
        } else {
            await transferStore.stopWebUpload()
        }
    } catch (error) {
        console.error('[ReceiveSettingsCard] Web 上传操作失败:', error)
        webUploadEnabled.value = !value
    } finally {
        webUploadLoading.value = false
    }
}

// 复制上传链接
async function handleCopyLink() {
    if (transferStore.webUploadInfo?.url) {
        try {
            await navigator.clipboard.writeText(transferStore.webUploadInfo.url)
        } catch (e) {
            console.error('复制链接失败:', e)
        }
    }
}

// 组件卸载时停止 Web 上传
onUnmounted(async () => {
    if (transferStore.webUploadEnabled) {
        try {
            await transferStore.stopWebUpload()
        } catch (e) {
            console.error('停止 Web 上传失败:', e)
        }
    }
})
</script>

<style scoped>
.setting-item {
    padding: 8px 0;
}

.setting-label {
    font-weight: 500;
}

.qr-code-image {
    width: 200px;
    height: 200px;
    border-radius: 8px;
}
</style>

<style>
/* 全局样式：二维码 tooltip 主题适配 */
.qr-tooltip-light,
.qr-tooltip-dark {
    padding: 8px !important;
}

.qr-tooltip-light {
    background: rgb(var(--v-theme-surface)) !important;
    border: 1px solid rgba(0, 0, 0, 0.12);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

.qr-tooltip-dark {
    background: rgba(30, 30, 30, 0.95) !important;
    border: 1px solid rgba(255, 255, 255, 0.12);
}

.qr-tooltip-content {
    display: flex;
    align-items: center;
    justify-content: center;
}

.qr-code-tooltip-image {
    width: 150px;
    height: 150px;
    border-radius: 4px;
}

.qr-code-loading {
    padding: 16px;
    color: rgba(255, 255, 255, 0.7);
}
</style>
