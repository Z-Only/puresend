<template>
    <div>
        <v-card class="send-settings-card">
            <v-card-title class="text-subtitle-1">
                {{ t('send.settings.title') }}
            </v-card-title>
            <v-card-text>
                <!-- Web 下载开关 -->
                <div class="setting-item">
                    <div class="d-flex align-center justify-space-between">
                        <div>
                            <div class="setting-label text-body-1">
                                {{ t('send.settings.webDownload') }}
                            </div>
                            <div class="text-body-2 text-medium-emphasis">
                                {{ t('send.settings.webDownloadHint') }}
                            </div>
                        </div>
                        <v-switch
                            v-model="webDownloadEnabled"
                            color="primary"
                            hide-details
                            :loading="webDownloadLoading"
                            @update:model-value="handleWebDownloadChange"
                        />
                    </div>

                    <!-- Web 下载链接列表和二维码（开启后显示） -->
                    <div
                        v-if="
                            shareStore.isSharing &&
                            shareStore.shareLinks?.length
                        "
                        class="mt-4"
                    >
                        <div class="text-body-2 text-medium-emphasis mb-2">
                            {{ t('send.settings.webDownloadLink') }}:
                        </div>
                        <div
                            v-for="(shareLink, index) in shareStore.shareLinks"
                            :key="index"
                            class="url-item-row"
                        >
                            <code class="text-body-2 url-link">{{
                                shareLink
                            }}</code>
                            <div class="url-actions">
                                <v-btn
                                    :icon="mdiContentCopy"
                                    size="x-small"
                                    variant="text"
                                    @click="handleCopyUrl(shareLink)"
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
                                    <template
                                        #activator="{ props: tooltipProps }"
                                    >
                                        <v-btn
                                            v-bind="tooltipProps"
                                            :icon="mdiQrcode"
                                            size="x-small"
                                            variant="text"
                                        />
                                    </template>
                                    <div class="qr-tooltip-content">
                                        <img
                                            v-if="qrCodeDataUrls[shareLink]"
                                            :src="qrCodeDataUrls[shareLink]"
                                            :alt="
                                                t(
                                                    'send.settings.webDownloadQrcode'
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

                    <!-- 子设置项（Web 下载开启后显示） -->
                    <div v-if="shareStore.isSharing" class="mt-4 ml-4">
                        <!-- 自动接受请求 -->
                        <div
                            class="d-flex align-center justify-space-between mb-2"
                        >
                            <div>
                                <div class="text-body-2">
                                    {{ t('share.settings.autoAccept') }}
                                </div>
                                <div class="text-caption text-medium-emphasis">
                                    {{ t('share.settings.autoAcceptHint') }}
                                </div>
                            </div>
                            <v-switch
                                v-model="shareStore.settings.autoAccept"
                                color="primary"
                                hide-details
                                density="compact"
                                @update:model-value="handleAutoAcceptChange"
                            />
                        </div>
                        <!-- PIN 保护 -->
                        <div class="d-flex align-center justify-space-between">
                            <div>
                                <div class="text-body-2">
                                    {{ t('share.settings.pinEnabled') }}
                                </div>
                                <div class="text-caption text-medium-emphasis">
                                    <span
                                        v-if="
                                            shareStore.settings.pinEnabled &&
                                            shareStore.settings.pin
                                        "
                                    >
                                        {{ t('share.settings.currentPin') }}:
                                        <strong>{{
                                            shareStore.settings.pin
                                        }}</strong>
                                    </span>
                                    <span v-else>
                                        {{
                                            shareStore.settings.pinEnabled
                                                ? t('share.settings.pinSet')
                                                : t('share.settings.pinNotSet')
                                        }}
                                    </span>
                                </div>
                            </div>
                            <v-btn
                                size="small"
                                variant="text"
                                @click="showPinConfig = true"
                            >
                                {{ t('share.settings.configure') }}
                            </v-btn>
                        </div>
                    </div>
                </div>
            </v-card-text>
        </v-card>

        <!-- PIN 配置对话框 -->
        <PinConfigDialog
            :visible="showPinConfig"
            :current-pin="shareStore.settings.pin"
            :pin-enabled="shareStore.settings.pinEnabled"
            @update:visible="showPinConfig = $event"
            @confirm="handleConfirmPin"
        />

        <!-- 复制成功提示 -->
        <v-snackbar
            v-model="showCopied"
            :timeout="2000"
            color="success"
            location="top"
        >
            {{ t('share.link.copied') }}
        </v-snackbar>
    </div>
</template>

<script setup lang="ts">
/**
 * 发送设置卡片组件
 *
 * 展示发送相关的设置项：
 * - Web 下载开关（启动 HTTP 服务器供其他设备下载）
 * - 自动接受请求开关
 * - PIN 保护配置
 */
import { ref, watch, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTheme } from 'vuetify'
import { useShareStore } from '@/stores/share'
import { useTransferStore } from '@/stores/transfer'
import PinConfigDialog from './PinConfigDialog.vue'
import { mdiContentCopy, mdiQrcode } from '@mdi/js'
import QRCode from 'qrcode'
import {
    copyToClipboard,
    updateShareSettingsService,
} from '@/services/shareService'

const { t } = useI18n()
const vuetifyTheme = useTheme()
const shareStore = useShareStore()
const transferStore = useTransferStore()

// 当前是否为深色主题
const isDarkTheme = computed(() => vuetifyTheme.global.current.value.dark)

// 本地状态
const webDownloadEnabled = ref(shareStore.isSharing)
const webDownloadLoading = ref(false)
const qrCodeDataUrls = ref<Record<string, string>>({})
const showPinConfig = ref(false)
const showCopied = ref(false)

// 监听 shareStore.isSharing 同步 webDownloadEnabled
watch(
    () => shareStore.isSharing,
    (val) => {
        webDownloadEnabled.value = val
    },
    { immediate: true }
)

// 监听已选文件变化，自动同步到后端分享服务器
watch(
    () => shareStore.selectedFiles,
    async (files) => {
        if (!shareStore.isSharing) return
        const filesToSync = files.map((f) => ({
            id: f.id,
            name: f.name,
            size: f.size,
            mimeType: f.mimeType || 'application/octet-stream',
            hash: '',
            chunks: [],
            path: f.path,
        }))
        await shareStore.updateShareFiles(filesToSync)
    },
    { deep: true }
)

// 监听 shareStore.shareLinks 生成所有链接的二维码
watch(
    () => shareStore.shareLinks,
    async (links) => {
        if (links?.length) {
            const newQrCodes: Record<string, string> = {}
            for (const link of links) {
                try {
                    newQrCodes[link] = await QRCode.toDataURL(link, {
                        width: 200,
                        margin: 2,
                    })
                } catch (e) {
                    console.error('[SendSettingsCard] 生成二维码失败:', e)
                }
            }
            qrCodeDataUrls.value = newQrCodes
        } else {
            qrCodeDataUrls.value = {}
        }
    }
)

// 复制指定链接
async function handleCopyUrl(url: string) {
    try {
        await copyToClipboard(url)
        showCopied.value = true
    } catch (e) {
        console.error('复制链接失败:', e)
    }
}

// 处理 Web 下载开关变化
async function handleWebDownloadChange(value: boolean | null) {
    if (value === null) return

    webDownloadLoading.value = true
    try {
        if (value) {
            // 开启：启动分享服务，传入已选文件列表
            const filesToShare = shareStore.selectedFiles.map((f) => ({
                id: f.id,
                name: f.name,
                size: f.size,
                mimeType: f.mimeType || 'application/octet-stream',
                hash: '',
                chunks: [],
                path: f.path,
            }))
            const result = await shareStore.startShare(filesToShare)
            if (!result) {
                // startShare 返回 null 表示失败，回滚开关状态
                webDownloadEnabled.value = false
                return
            }
            transferStore.webDownloadEnabled = true
        } else {
            // 关闭：停止分享服务
            await shareStore.stopShare()
            transferStore.webDownloadEnabled = false
        }
    } catch (error) {
        console.error('[SendSettingsCard] Web 下载操作失败:', error)
        // 恢复开关状态
        webDownloadEnabled.value = !value
    } finally {
        webDownloadLoading.value = false
    }
}

// 处理自动接受开关变化
async function handleAutoAcceptChange(value: boolean | null) {
    if (value !== null) {
        shareStore.settings.autoAccept = value
        await updateShareSettingsService(shareStore.settings)
    }
}

// 处理 PIN 配置确认
async function handleConfirmPin(pin: string) {
    shareStore.settings.pin = pin
    shareStore.settings.pinEnabled = true
    await updateShareSettingsService(shareStore.settings)
    showPinConfig.value = false
}
</script>

<style scoped>
.send-settings-card {
    height: auto;
}

.setting-item {
    padding: 4px 0;
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

.qr-tooltip-content {
    padding: 8px;
    background: white;
    border-radius: 8px;
}

.qr-code-tooltip-image {
    display: block;
    width: 200px;
    height: 200px;
    border-radius: 4px;
}

.qr-code-loading {
    width: 200px;
    height: 200px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #666;
    font-size: 14px;
}
</style>
