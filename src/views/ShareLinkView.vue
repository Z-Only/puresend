<!-- 链接分享界面 -->
<template>
    <v-container fluid class="share-link-view">
        <v-row>
            <v-col cols="12" md="6">
                <!-- 返回按钮 -->
                <v-btn
                    variant="text"
                    :prepend-icon="mdiArrowLeft"
                    @click="handleBack"
                    class="mb-4"
                >
                    {{ t('share.backToSend') }}
                </v-btn>

                <!-- 分享链接面板 -->
                <LinkSharePanel @settings="handleShareSettings" />
            </v-col>

            <v-col cols="12" md="6">
                <!-- 分享设置卡片 -->
                <v-card class="mb-4">
                    <v-card-title>{{ t('share.settings.title') }}</v-card-title>
                    <v-card-text>
                        <v-list>
                            <v-list-item>
                                <v-list-item-title>
                                    {{ t('share.settings.autoAccept') }}
                                </v-list-item-title>
                                <v-list-item-subtitle>
                                    {{
                                        shareStore.settings.autoAccept
                                            ? t('common.enabled')
                                            : t('common.disabled')
                                    }}
                                </v-list-item-subtitle>
                                <template v-slot:append>
                                    <v-switch
                                        v-model="shareStore.settings.autoAccept"
                                        @update:model-value="
                                            handleApplyShareSettings(
                                                shareStore.settings
                                            )
                                        "
                                        density="compact"
                                    />
                                </template>
                            </v-list-item>
                            <v-list-item>
                                <v-list-item-title>
                                    {{ t('share.settings.pinEnabled') }}
                                </v-list-item-title>
                                <v-list-item-subtitle>
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
                                </v-list-item-subtitle>
                                <template v-slot:append>
                                    <v-btn
                                        size="small"
                                        variant="text"
                                        @click="handlePinConfig"
                                    >
                                        {{ t('share.settings.configure') }}
                                    </v-btn>
                                </template>
                            </v-list-item>
                        </v-list>
                    </v-card-text>
                </v-card>

                <!-- 访问请求和下载进度 -->
                <v-card class="mb-4">
                    <v-card-title>
                        {{ t('share.requests.title') }}
                        <v-chip
                            v-if="shareStore.pendingRequests.length > 0"
                            color="error"
                            size="small"
                            class="ml-2"
                        >
                            {{ shareStore.pendingRequests.length }}
                        </v-chip>
                    </v-card-title>
                    <v-card-text>
                        <v-list v-if="shareStore.accessRequests.size > 0">
                            <v-list-item
                                v-for="request in shareStore.accessRequests.values()"
                                :key="request.id"
                            >
                                <v-list-item-title>
                                    {{ request.ip }}
                                    <span
                                        v-if="request.userAgent"
                                        class="text-grey ml-2"
                                    >
                                        {{ request.userAgent }}
                                    </span>
                                </v-list-item-title>
                                <v-list-item-subtitle>
                                    {{ formatTime(request.requestedAt) }}
                                </v-list-item-subtitle>
                                <template v-slot:append>
                                    <!-- 待处理状态：显示同意/拒绝按钮 -->
                                    <template
                                        v-if="request.status === 'pending'"
                                    >
                                        <v-btn
                                            :icon="mdiCheck"
                                            size="small"
                                            variant="text"
                                            color="success"
                                            @click="
                                                handleAcceptRequest(request.id)
                                            "
                                        />
                                        <v-btn
                                            :icon="mdiClose"
                                            size="small"
                                            variant="text"
                                            color="error"
                                            @click="
                                                handleRejectRequest(request.id)
                                            "
                                        />
                                    </template>
                                    <!-- 已处理状态：显示状态标签 -->
                                    <template v-else>
                                        <v-chip
                                            :color="
                                                request.status === 'accepted'
                                                    ? 'success'
                                                    : 'error'
                                            "
                                            size="small"
                                            label
                                        >
                                            {{
                                                request.status === 'accepted'
                                                    ? t(
                                                          'share.requests.status.accepted'
                                                      )
                                                    : t(
                                                          'share.requests.status.rejected'
                                                      )
                                            }}
                                        </v-chip>
                                    </template>
                                </template>
                            </v-list-item>
                        </v-list>
                        <div v-else class="text-body-2 text-grey">
                            {{ t('share.noRequests') }}
                        </div>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>

        <!-- 分享设置对话框 -->
        <ShareSettingsDialog
            :visible="showShareSettings"
            :settings="shareSettings"
            @update:visible="showShareSettings = $event"
            @update:settings="handleApplyShareSettings"
        />

        <!-- PIN 配置对话框 -->
        <PinConfigDialog
            :visible="showPinConfig"
            :current-pin="shareStore.settings.pin"
            :pin-enabled="shareStore.settings.pinEnabled"
            @update:visible="showPinConfig = $event"
            @confirm="handleConfirmPin"
        />

        <!-- 错误提示 -->
        <v-snackbar v-model="showError" color="error" :timeout="5000">
            {{ errorMessage }}
        </v-snackbar>
    </v-container>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useShareStore, useTransferStore } from '../stores'
import {
    LinkSharePanel,
    ShareSettingsDialog,
    PinConfigDialog,
} from '../components/transfer'
import { updateShareSettingsService } from '../services/shareService'
import type { ShareSettings } from '../types'
import { mdiArrowLeft, mdiCheck, mdiClose } from '@mdi/js'

const router = useRouter()
const { t } = useI18n()
const shareStore = useShareStore()
const transferStore = useTransferStore()

const showError = ref(false)
const errorMessage = ref('')
const showShareSettings = ref(false)
const showPinConfig = ref(false)
const shareSettings = ref<ShareSettings>({
    pinEnabled: false,
    pin: '',
    autoAccept: false,
})

// 格式化时间
function formatTime(timestamp: number): string {
    return new Date(timestamp).toLocaleTimeString()
}

// 返回发送页面
function handleBack() {
    // 停止分享
    if (shareStore.isSharing) {
        shareStore.stopShare()
    }
    // 只重置分享相关状态，保留已选文件
    shareStore.shareInfo = null
    shareStore.accessRequests.clear()
    shareStore.qrCodeDataUrl = ''
    // 重置发送模式为默认的 p2p 模式
    transferStore.setSendMode('p2p')
    // 返回发送页面
    router.push({ name: 'Send' })
}

// 打开分享设置
function handleShareSettings() {
    showShareSettings.value = true
}

// 打开 PIN 配置
function handlePinConfig() {
    showPinConfig.value = true
}

// 应用分享设置
async function handleApplyShareSettings(settings: ShareSettings) {
    shareSettings.value = settings
    shareStore.updateSettings(settings)

    // 同步设置到后端
    try {
        await updateShareSettingsService(settings)
    } catch (error) {
        console.error('更新分享设置失败:', error)
        showError.value = true
        errorMessage.value = t('share.settingsUpdateError', { error })
    }
}

// 确认 PIN 配置
async function handleConfirmPin(pin: string) {
    // 更新 PIN 设置
    const newSettings: ShareSettings = {
        ...shareStore.settings,
        pinEnabled: true,
        pin: pin,
    }
    shareStore.updateSettings(newSettings)
    shareSettings.value = newSettings

    // 同步设置到后端
    try {
        await updateShareSettingsService(newSettings)
    } catch (error) {
        console.error('更新分享设置失败:', error)
        showError.value = true
        errorMessage.value = t('share.settingsUpdateError', { error })
    }
}

// 接受访问请求
async function handleAcceptRequest(requestId: string) {
    try {
        await shareStore.acceptRequest(requestId)
    } catch (error) {
        showError.value = true
        errorMessage.value = t('share.acceptError', { error })
    }
}

// 拒绝访问请求
async function handleRejectRequest(requestId: string) {
    try {
        await shareStore.rejectRequest(requestId)
    } catch (error) {
        showError.value = true
        errorMessage.value = t('share.rejectError', { error })
    }
}

// 监听事件
onMounted(async () => {
    // 自动开始分享（使用 store 中的方法，事件监听已在 store 中设置）
    if (shareStore.selectedFiles.length > 0 && !shareStore.isSharing) {
        try {
            const files = shareStore.selectedFiles.map((f) => ({
                id: f.path,
                name: f.name,
                size: f.size,
                mimeType: f.mimeType,
                hash: '',
                chunks: [],
                path: f.path,
            }))
            await shareStore.startShare(files)
        } catch (error) {
            console.error('自动开始分享失败:', error)
            showError.value = true
            errorMessage.value = t('share.startError', { error })
        }
    }
})

// 清理
onUnmounted(() => {
    // 事件监听器由 store 管理，这里不需要手动清理
})
</script>

<style scoped>
.share-link-view {
    max-width: 1200px;
    margin: 0 auto;
}
</style>
