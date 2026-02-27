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

                <!-- Web 上传链接列表和二维码 -->
                <div
                    v-if="
                        transferStore.webUploadEnabled &&
                        transferStore.webUploadInfo?.urls?.length
                    "
                    class="mt-4"
                >
                    <div class="text-body-2 text-medium-emphasis mb-2">
                        {{ t('receive.settings.webUploadLink') }}:
                    </div>
                    <div
                        v-for="(uploadUrl, index) in transferStore.webUploadInfo
                            .urls"
                        :key="index"
                        class="url-item-row"
                    >
                        <code class="text-body-2 url-link">{{
                            uploadUrl
                        }}</code>
                        <div class="url-actions">
                            <v-btn
                                :icon="mdiContentCopy"
                                size="x-small"
                                variant="text"
                                @click="handleCopyUrl(uploadUrl)"
                            />
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
                                        @click="generateQrForUrl(uploadUrl)"
                                    />
                                </template>
                                <div class="qr-tooltip-content">
                                    <img
                                        v-if="qrCodeDataUrls[uploadUrl]"
                                        :src="qrCodeDataUrls[uploadUrl]"
                                        :alt="
                                            t(
                                                'receive.settings.webUploadQrcode'
                                            )
                                        "
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
import { ref, watch, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTheme } from 'vuetify'
import { open } from '@tauri-apps/plugin-dialog'
import { useTransferStore } from '@/stores/transfer'
import { useSettingsStore } from '@/stores/settings'
import { mdiFolderOpen, mdiContentCopy, mdiQrcode } from '@mdi/js'
import QRCode from 'qrcode'
// 移动端平台检测
import { usePlatform } from '@/composables'
// 移动端文件系统 API
import { AndroidFs } from 'tauri-plugin-android-fs-api'

const { t } = useI18n()
const vuetifyTheme = useTheme()
const transferStore = useTransferStore()
const settingsStore = useSettingsStore()
const { isMobile } = usePlatform()

// 当前是否为深色主题
const isDarkTheme = computed(() => vuetifyTheme.global.current.value.dark)

// 本地状态（与 store 同步）
const autoReceive = ref(settingsStore.receiveSettings.autoReceive)
const fileOverwrite = ref(settingsStore.receiveSettings.fileOverwrite)
const webUploadEnabled = ref(transferStore.webUploadEnabled)
const webUploadLoading = ref(false)
const qrCodeDataUrls = ref<Record<string, string>>({})

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
    },
    { immediate: true }
)

watch(
    () => transferStore.webUploadInfo,
    async (info) => {
        if (info?.urls?.length) {
            const newQrCodes: Record<string, string> = {}
            for (const url of info.urls) {
                try {
                    newQrCodes[url] = await QRCode.toDataURL(url, {
                        width: 200,
                        margin: 2,
                    })
                } catch (e) {
                    console.error('生成二维码失败:', e)
                }
            }
            qrCodeDataUrls.value = newQrCodes
        } else {
            qrCodeDataUrls.value = {}
        }
    },
    { immediate: true }
)

// 为指定 URL 生成二维码（用于点击时触发）
async function generateQrForUrl(url: string) {
    if (qrCodeDataUrls.value[url]) return
    try {
        qrCodeDataUrls.value[url] = await QRCode.toDataURL(url, {
            width: 200,
            margin: 2,
        })
    } catch (e) {
        console.error('生成二维码失败:', e)
    }
}

// 复制指定 URL
async function handleCopyUrl(url: string) {
    try {
        await navigator.clipboard.writeText(url)
    } catch (e) {
        console.error('复制失败:', e)
    }
}

// 选择接收目录
async function handleSelectDirectory() {
    try {
        if (isMobile.value) {
            // 移动端：使用 Android 文件系统插件
            const dirUri = await AndroidFs.showOpenDirPicker()

            if (dirUri) {
                // 移动端使用 URI 作为路径
                await transferStore.updateReceiveDirectory(dirUri)
            }
        } else {
            // 桌面端：使用 Tauri 原生对话框
            const selected = await open({
                directory: true,
                multiple: false,
                title: t('receive.selectDirectory'),
            })

            if (selected && typeof selected === 'string') {
                await transferStore.updateReceiveDirectory(selected)
            }
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

</script>

<style scoped>
.setting-item {
    padding: 8px 0;
}

.setting-label {
    font-weight: 500;
}

.url-item-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 4px;
    gap: 8px;
}

.url-link {
    flex: 1;
    min-width: 0;
    word-break: break-all;
    text-align: left;
}

.url-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
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
