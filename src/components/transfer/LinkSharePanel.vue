<script setup lang="ts">
/**
 * 分享链接面板组件
 *
 * 显示分享链接、二维码和访问控制功能
 */
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTheme } from 'vuetify'
import { storeToRefs } from 'pinia'
import { useShareStore } from '@/stores/share'
import { copyToClipboard, generateQRCode } from '@/services/shareService'
import { mdiContentCopy, mdiQrcode } from '@mdi/js'

const { t } = useI18n()
const vuetifyTheme = useTheme()
const shareStore = useShareStore()
const { shareInfo, isSharing, qrCodeDataUrl } = storeToRefs(shareStore)

// 当前是否为深色主题
const isDarkTheme = computed(() => vuetifyTheme.global.current.value.dark)

// 本地状态
const copiedLink = ref(false)
const showSnackbar = ref(false)
const snackbarMessage = ref('')

// 计算属性 - 直接使用后端返回的完整链接
const shareLink = computed(() => {
    if (!shareInfo.value) return ''
    // 后端已返回完整 URL 格式，直接使用
    return shareInfo.value.link
})
// 复制链接
async function handleCopyLink() {
    if (!shareLink.value) return
    await copyToClipboard(shareLink.value)
    copiedLink.value = true
    snackbarMessage.value = t('share.link.copied')
    showSnackbar.value = true
    setTimeout(() => {
        copiedLink.value = false
    }, 2000)
}

// 格式化时间
function formatTime(timestamp: number): string {
    return new Date(timestamp).toLocaleTimeString()
}

// 生成二维码
onMounted(async () => {
    if (shareLink.value) {
        try {
            const qrUrl = await generateQRCode(shareLink.value)
            shareStore.setQRCode(qrUrl)
        } catch (error) {
            console.error('生成二维码失败:', error)
        }
    }
})

// 监听链接变化，重新生成二维码
watch(shareLink, async (newLink) => {
    if (newLink) {
        try {
            const qrUrl = await generateQRCode(newLink)
            shareStore.setQRCode(qrUrl)
        } catch (error) {
            console.error('生成二维码失败:', error)
        }
    }
})
</script>

<template>
    <div class="link-share-panel">
        <!-- 分享状态 -->
        <div v-if="isSharing && shareInfo" class="share-active">
            <!-- 分享链接区域 -->
            <div class="share-link-section">
                <div class="section-title">{{ t('share.link.title') }}</div>
                <div class="link-display">
                    <span class="link-text">{{ shareLink }}</span>
                    <div class="action-buttons">
                        <v-btn
                            :icon="mdiContentCopy"
                            :class="{ copied: copiedLink }"
                            @click="handleCopyLink"
                            :title="
                                copiedLink
                                    ? t('share.link.copied')
                                    : t('share.link.copy')
                            "
                            variant="text"
                            size="small"
                        />
                        <!-- 使用 v-tooltip 实现点击或悬停显示二维码 -->
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
                                    variant="text"
                                    size="small"
                                />
                            </template>
                            <div class="qr-tooltip-content">
                                <img
                                    v-if="qrCodeDataUrl"
                                    :src="qrCodeDataUrl"
                                    alt="QR Code"
                                    class="qr-code-tooltip-image"
                                />
                                <div v-else class="qr-code-loading">
                                    {{ t('share.qrcode.generating') }}
                                </div>
                            </div>
                        </v-tooltip>
                    </div>
                </div>
                <div class="share-warning">
                    ⚠️ {{ t('share.link.warning') }}
                </div>
            </div>

            <!-- 复制成功提示 -->
            <v-snackbar
                v-model="showSnackbar"
                :timeout="2000"
                color="success"
                location="top"
            >
                {{ snackbarMessage }}
            </v-snackbar>

            <!-- 分享信息 -->
            <div class="share-info-section">
                <div class="info-item">
                    <span class="info-label">{{ t('share.info.files') }}</span>
                    <span class="info-value">{{ shareInfo.files.length }}</span>
                </div>
                <div class="info-item">
                    <span class="info-label">{{
                        t('share.info.created')
                    }}</span>
                    <span class="info-value">{{
                        formatTime(shareInfo.createdAt)
                    }}</span>
                </div>
                <div class="info-item">
                    <span class="info-label">{{ t('share.info.status') }}</span>
                    <span class="info-value status-active">{{
                        t('share.status.active')
                    }}</span>
                </div>
            </div>

            <!-- PIN 设置提示 -->
            <div v-if="shareInfo.pinEnabled" class="pin-info">
                🔒 {{ t('share.pin.enabled') }}
            </div>
        </div>
    </div>
</template>

<style scoped>
.link-share-panel {
    padding: 20px;
    max-width: 600px;
    margin: 0 auto;
}

.section-title {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 12px;
}

/* 分享链接区域 */
.share-link-section {
    background: var(--bg-secondary);
    border-radius: 12px;
    padding: 16px;
    margin-bottom: 20px;
}

.link-display {
    display: flex;
    align-items: center;
    gap: 12px;
    background: var(--bg-primary);
    border-radius: 8px;
    padding: 12px;
    margin-bottom: 12px;
}

.link-text {
    flex: 1;
    font-size: 15px;
    color: var(--text-primary);
    word-break: break-all;
}

.action-buttons {
    display: flex;
    gap: 8px;
}

.icon-btn.copied {
    color: var(--success-color) !important;
}

/* 二维码 tooltip 样式 */
.qr-tooltip-content {
    padding: 12px;
    background: var(--bg-primary);
    border-radius: 8px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
}

.qr-code-tooltip-image {
    width: 180px;
    height: 180px;
    display: block;
}

.qr-code-loading {
    width: 180px;
    height: 180px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
}

/* 深色主题适配 */
.qr-tooltip-dark {
    background: var(--bg-secondary) !important;
    border: 1px solid var(--border-color) !important;
}

.qr-tooltip-light {
    background: var(--bg-primary) !important;
    border: 1px solid var(--border-color-light) !important;
}

.qrcode-image {
    width: 180px;
    height: 180px;
}

.qrcode-loading {
    width: 180px;
    height: 180px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
}

.share-warning {
    font-size: 13px;
    color: var(--warning-color);
    padding: 8px 12px;
    background: var(--warning-bg);
    border-radius: 6px;
}

/* 二维码区域 */
.qrcode-section {
    text-align: center;
    margin-bottom: 20px;
}

.qrcode-container {
    display: inline-block;
    padding: 16px;
    border-radius: 12px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.qrcode-image {
    width: 180px;
    height: 180px;
}

.qrcode-loading {
    width: 180px;
    height: 180px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
}

/* 分享信息 */
.share-info-section {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
    margin-bottom: 20px;
}

.info-item {
    text-align: center;
    padding: 12px;
    background: var(--bg-secondary);
    border-radius: 8px;
}

.info-label {
    display: block;
    font-size: 13px;
    color: var(--text-secondary);
    margin-bottom: 4px;
}

.info-value {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
}

.status-active {
    color: var(--success-color);
}

/* PIN 信息 */
.pin-info {
    padding: 12px;
    background: var(--info-bg);
    border-radius: 8px;
    font-size: 14px;
    color: var(--info-color);
    margin-bottom: 20px;
}
</style>
