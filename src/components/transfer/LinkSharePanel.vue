<script setup lang="ts">
/**
 * 分享链接面板组件
 *
 * 显示分享链接、二维码和访问控制功能
 */
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { storeToRefs } from 'pinia'
import { useShareStore } from '@/stores/share'
import {
    copyToClipboard,
    generateQRCode,
    onDownloadProgress,
    type DownloadProgress,
} from '@/services/shareService'
import type { UnlistenFn } from '@tauri-apps/api/event'

const { t } = useI18n()
const shareStore = useShareStore()
const { shareInfo, isSharing, qrCodeDataUrl } = storeToRefs(shareStore)

// 本地状态
const copiedLink = ref(false)
const unlistenFns: UnlistenFn[] = []

// 计算属性 - 直接使用后端返回的完整链接
const shareLink = computed(() => {
    if (!shareInfo.value) return ''
    // 后端已返回完整 URL 格式，直接使用
    return shareInfo.value.link
})
const activeDownloads = ref<Map<string, DownloadProgress>>(new Map())

// 复制链接
async function handleCopyLink() {
    if (!shareLink.value) return
    await copyToClipboard(shareLink.value)
    copiedLink.value = true
    setTimeout(() => {
        copiedLink.value = false
    }, 2000)
}

// 格式化时间
function formatTime(timestamp: number): string {
    return new Date(timestamp).toLocaleTimeString()
}

// 监听事件
onMounted(async () => {
    // 监听下载进度
    const unlistenProgress = await onDownloadProgress(
        (progress: DownloadProgress) => {
            activeDownloads.value.set(progress.downloadId, progress)
            if (progress.progress >= 100) {
                setTimeout(() => {
                    activeDownloads.value.delete(progress.downloadId)
                }, 3000)
            }
        }
    )
    unlistenFns.push(unlistenProgress)

    // 生成二维码
    if (shareLink.value) {
        try {
            const qrUrl = await generateQRCode(shareLink.value)
            shareStore.setQRCode(qrUrl)
        } catch (error) {
            console.error('生成二维码失败:', error)
        }
    }
})

// 清理事件监听
onUnmounted(() => {
    unlistenFns.forEach((fn) => fn())
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
                    <button
                        class="copy-btn"
                        :class="{ copied: copiedLink }"
                        @click="handleCopyLink"
                    >
                        {{
                            copiedLink
                                ? t('share.link.copied')
                                : t('share.link.copy')
                        }}
                    </button>
                </div>
                <div class="share-warning">
                    ⚠️ {{ t('share.link.warning') }}
                </div>
            </div>

            <!-- 二维码区域 -->
            <div class="qrcode-section">
                <div class="section-title">{{ t('share.qrcode.title') }}</div>
                <div class="qrcode-container">
                    <img
                        v-if="qrCodeDataUrl"
                        :src="qrCodeDataUrl"
                        alt="QR Code"
                        class="qrcode-image"
                    />
                    <div v-else class="qrcode-loading">
                        {{ t('share.qrcode.generating') }}
                    </div>
                </div>
            </div>

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

            <!-- 下载进度 -->
            <div v-if="activeDownloads.size > 0" class="downloads-section">
                <div class="section-title">
                    {{ t('share.downloads.title') }}
                </div>
                <div class="downloads-list">
                    <div
                        v-for="[id, progress] in activeDownloads"
                        :key="id"
                        class="download-item"
                    >
                        <div class="download-info">
                            <span class="download-file">{{
                                progress.fileName
                            }}</span>
                            <span class="download-client">{{
                                progress.clientIp
                            }}</span>
                        </div>
                        <div class="download-progress">
                            <div class="progress-bar">
                                <div
                                    class="progress-fill"
                                    :style="{ width: `${progress.progress}%` }"
                                ></div>
                            </div>
                            <span class="progress-text"
                                >{{ progress.progress.toFixed(1) }}%</span
                            >
                        </div>
                    </div>
                </div>
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
    font-size: 14px;
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
    font-size: 14px;
    color: var(--text-primary);
    word-break: break-all;
}

.copy-btn {
    padding: 8px 16px;
    background: var(--primary-color);
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    transition: all 0.2s;
    font-weight: 500;
}

.copy-btn:hover {
    background: var(--primary-color-dark);
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

.copy-btn.copied {
    background: var(--success-color);
}

.share-warning {
    font-size: 12px;
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
    font-size: 12px;
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
    font-size: 13px;
    color: var(--info-color);
    margin-bottom: 20px;
}

/* 访问请求 */
.access-requests-section {
    margin-bottom: 20px;
}

.requests-list {
    max-height: 200px;
    overflow-y: auto;
}

.request-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px;
    background: var(--bg-secondary);
    border-radius: 8px;
    margin-bottom: 8px;
}

.request-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.request-ip {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
}

.request-time {
    font-size: 12px;
    color: var(--text-secondary);
}

.request-actions {
    display: flex;
    gap: 8px;
}

.accept-btn,
.reject-btn {
    padding: 6px 12px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
}

.accept-btn {
    background: var(--success-color);
    color: white;
}

.reject-btn {
    background: var(--danger-color);
    color: white;
}

/* 下载进度 */
.downloads-section {
    margin-bottom: 20px;
}

.download-item {
    padding: 12px;
    background: var(--bg-secondary);
    border-radius: 8px;
    margin-bottom: 8px;
}

.download-info {
    display: flex;
    justify-content: space-between;
    margin-bottom: 8px;
}

.download-file {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
}

.download-client {
    font-size: 12px;
    color: var(--text-secondary);
}

.download-progress {
    display: flex;
    align-items: center;
    gap: 8px;
}

.progress-bar {
    flex: 1;
    height: 4px;
    background: var(--bg-primary);
    border-radius: 2px;
    overflow: hidden;
}

.progress-fill {
    height: 100%;
    background: var(--primary-color);
    transition: width 0.3s;
}

.progress-text {
    font-size: 12px;
    color: var(--text-secondary);
    min-width: 48px;
    text-align: right;
}

/* 停止按钮 */
.actions-section {
    text-align: center;
}

.stop-btn {
    padding: 12px 32px;
    background: var(--danger-color);
    color: white;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
}

.stop-btn:hover {
    background: var(--danger-color-dark);
}

/* 未分享状态 */
.share-inactive {
    text-align: center;
    padding: 40px 20px;
}

.inactive-icon {
    font-size: 48px;
    margin-bottom: 16px;
}

.inactive-text {
    font-size: 16px;
    color: var(--text-primary);
    margin-bottom: 8px;
}

.inactive-hint {
    font-size: 13px;
    color: var(--text-secondary);
}
</style>
