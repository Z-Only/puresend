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
                    class="mb-4 back-btn"
                    style="font-size: 16px"
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
                    <v-card-title
                        class="d-flex align-center justify-space-between"
                    >
                        <div>
                            {{ t('share.requests.title') }}
                            <v-chip
                                v-if="shareStore.pendingRequests.length > 0"
                                color="error"
                                size="small"
                                class="ml-2"
                            >
                                {{ shareStore.pendingRequests.length }}
                            </v-chip>
                        </div>
                        <v-btn
                            v-if="shareStore.accessRequests.size > 0"
                            :prepend-icon="mdiDeleteSweep"
                            variant="text"
                            size="small"
                            color="error"
                            @click="showClearAllDialog = true"
                        >
                            {{ t('share.requests.clearAll') }}
                        </v-btn>
                    </v-card-title>
                    <v-card-text>
                        <v-list v-if="shareStore.accessRequests.size > 0">
                            <template
                                v-for="(request, index) in Array.from(
                                    shareStore.accessRequests.values()
                                )"
                                :key="request.id"
                            >
                                <v-list-item>
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
                                        <template
                                            v-if="
                                                request.uploadRecords &&
                                                request.uploadRecords.length > 1
                                            "
                                        >
                                            ·
                                            {{
                                                t('share.uploads.fileCount', {
                                                    count: request.uploadRecords
                                                        .length,
                                                })
                                            }}
                                            ·
                                            {{
                                                formatFileSize(
                                                    request.uploadRecords.reduce(
                                                        (sum, r) =>
                                                            sum + r.totalBytes,
                                                        0
                                                    )
                                                )
                                            }}
                                        </template>
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
                                                    handleAcceptRequest(
                                                        request.id
                                                    )
                                                "
                                            />
                                            <v-btn
                                                :icon="mdiClose"
                                                size="small"
                                                variant="text"
                                                color="error"
                                                @click="
                                                    handleRejectRequest(
                                                        request.id
                                                    )
                                                "
                                            />
                                        </template>
                                        <!-- 已接受状态 -->
                                        <template
                                            v-else-if="
                                                request.status === 'accepted'
                                            "
                                        >
                                            <v-chip
                                                color="success"
                                                size="small"
                                                label
                                            >
                                                {{
                                                    t(
                                                        'share.requests.status.accepted'
                                                    )
                                                }}
                                            </v-chip>
                                        </template>
                                        <!-- 已拒绝状态 -->
                                        <template v-else>
                                            <v-chip
                                                color="error"
                                                size="small"
                                                label
                                            >
                                                {{
                                                    t(
                                                        'share.requests.status.rejected'
                                                    )
                                                }}
                                            </v-chip>
                                        </template>
                                        <!-- 移除按钮 -->
                                        <v-btn
                                            :icon="mdiDelete"
                                            size="small"
                                            variant="text"
                                            color="grey"
                                            @click="
                                                showRemoveDialog(request.id)
                                            "
                                        />
                                    </template>
                                </v-list-item>

                                <!-- 上传记录列表 -->
                                <div
                                    v-if="
                                        request.uploadRecords &&
                                        request.uploadRecords.length > 0
                                    "
                                    class="upload-records-container ml-4 mr-4 mb-2"
                                >
                                    <!-- 折叠状态：显示前 3 条 -->
                                    <template v-if="!isExpanded(request.id)">
                                        <div
                                            v-for="(
                                                record, index
                                            ) in request.uploadRecords.slice(
                                                0,
                                                3
                                            )"
                                            :key="record.id"
                                            :class="[
                                                'upload-record-item',
                                                {
                                                    'has-divider':
                                                        index <
                                                        request.uploadRecords.slice(
                                                            0,
                                                            3
                                                        ).length -
                                                            1,
                                                },
                                            ]"
                                        >
                                            <div
                                                class="d-flex align-center justify-space-between"
                                            >
                                                <div
                                                    class="d-flex align-center flex-grow-1 text-truncate"
                                                    style="max-width: 65%"
                                                >
                                                    <span
                                                        class="text-body-2 text-truncate"
                                                    >
                                                        {{ record.fileName }}
                                                    </span>
                                                    <span
                                                        class="text-caption text-grey ml-2"
                                                        style="
                                                            white-space: nowrap;
                                                        "
                                                    >
                                                        {{
                                                            formatFileSize(
                                                                record.totalBytes
                                                            )
                                                        }}
                                                    </span>
                                                </div>
                                                <div
                                                    class="d-flex align-center ga-2"
                                                >
                                                    <span
                                                        v-if="
                                                            record.status ===
                                                            'transferring'
                                                        "
                                                        class="text-body-2 text-grey"
                                                    >
                                                        {{
                                                            formatSpeed(
                                                                record.speed
                                                            )
                                                        }}
                                                    </span>
                                                    <span class="text-body-2">
                                                        {{
                                                            record.progress.toFixed(
                                                                1
                                                            )
                                                        }}%
                                                    </span>
                                                    <v-chip
                                                        :color="
                                                            getUploadStatusColor(
                                                                record.status
                                                            )
                                                        "
                                                        size="x-small"
                                                        label
                                                    >
                                                        {{
                                                            getUploadStatusText(
                                                                record.status
                                                            )
                                                        }}
                                                    </v-chip>
                                                </div>
                                            </div>
                                            <v-progress-linear
                                                v-if="record.status !== 'idle'"
                                                :model-value="record.progress"
                                                :color="
                                                    record.status ===
                                                    'completed'
                                                        ? 'success'
                                                        : record.status ===
                                                            'failed'
                                                          ? 'error'
                                                          : 'primary'
                                                "
                                                height="3"
                                                class="mt-1"
                                            />
                                        </div>
                                    </template>

                                    <!-- 展开状态：显示全部记录（可滚动） -->
                                    <div
                                        v-show="isExpanded(request.id)"
                                        class="expanded-records"
                                    >
                                        <div
                                            v-for="(
                                                record, index
                                            ) in request.uploadRecords"
                                            :key="record.id"
                                            :class="[
                                                'upload-record-item',
                                                {
                                                    'has-divider':
                                                        index <
                                                        request.uploadRecords
                                                            .length -
                                                            1,
                                                },
                                            ]"
                                        >
                                            <div
                                                class="d-flex align-center justify-space-between"
                                            >
                                                <div
                                                    class="d-flex align-center flex-grow-1 text-truncate"
                                                    style="max-width: 65%"
                                                >
                                                    <span
                                                        class="text-body-2 text-truncate"
                                                    >
                                                        {{ record.fileName }}
                                                    </span>
                                                    <span
                                                        class="text-caption text-grey ml-2"
                                                        style="
                                                            white-space: nowrap;
                                                        "
                                                    >
                                                        {{
                                                            formatFileSize(
                                                                record.totalBytes
                                                            )
                                                        }}
                                                    </span>
                                                </div>
                                                <div
                                                    class="d-flex align-center ga-2"
                                                >
                                                    <span
                                                        v-if="
                                                            record.status ===
                                                            'transferring'
                                                        "
                                                        class="text-body-2 text-grey"
                                                    >
                                                        {{
                                                            formatSpeed(
                                                                record.speed
                                                            )
                                                        }}
                                                    </span>
                                                    <span class="text-body-2">
                                                        {{
                                                            record.progress.toFixed(
                                                                1
                                                            )
                                                        }}%
                                                    </span>
                                                    <v-chip
                                                        :color="
                                                            getUploadStatusColor(
                                                                record.status
                                                            )
                                                        "
                                                        size="x-small"
                                                        label
                                                    >
                                                        {{
                                                            getUploadStatusText(
                                                                record.status
                                                            )
                                                        }}
                                                    </v-chip>
                                                </div>
                                            </div>
                                            <v-progress-linear
                                                v-if="record.status !== 'idle'"
                                                :model-value="record.progress"
                                                :color="
                                                    record.status ===
                                                    'completed'
                                                        ? 'success'
                                                        : record.status ===
                                                            'failed'
                                                          ? 'error'
                                                          : 'primary'
                                                "
                                                height="3"
                                                class="mt-1"
                                            />
                                        </div>
                                    </div>

                                    <!-- 折叠/展开控件（超过 3 条时显示） -->
                                    <div
                                        v-if="request.uploadRecords.length > 3"
                                        class="text-center mt-1"
                                    >
                                        <v-btn
                                            variant="text"
                                            size="small"
                                            density="compact"
                                            @click="toggleExpand(request.id)"
                                        >
                                            <template
                                                v-if="!isExpanded(request.id)"
                                            >
                                                {{
                                                    t(
                                                        'share.uploads.moreRecords',
                                                        {
                                                            count:
                                                                request
                                                                    .uploadRecords
                                                                    .length - 3,
                                                        }
                                                    )
                                                }}
                                            </template>
                                            <template v-else>
                                                {{
                                                    t('share.uploads.collapse')
                                                }}
                                            </template>
                                        </v-btn>
                                    </div>
                                </div>

                                <!-- 只在不是最后一条请求时显示分隔线 -->
                                <v-divider
                                    v-if="
                                        index <
                                        Array.from(
                                            shareStore.accessRequests.values()
                                        ).length -
                                            1
                                    "
                                />
                            </template>
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

        <!-- 移除单个请求确认对话框 -->
        <v-dialog v-model="removeDialog" max-width="400">
            <v-card>
                <v-card-title>{{
                    t('share.removeConfirm.title')
                }}</v-card-title>
                <v-card-text>
                    {{ t('share.removeConfirm.message') }}
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn variant="text" @click="removeDialog = false">
                        {{ t('common.cancel') }}
                    </v-btn>
                    <v-btn color="error" variant="flat" @click="confirmRemove">
                        {{ t('common.remove') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

        <!-- 移除全部请求确认对话框 -->
        <v-dialog v-model="showClearAllDialog" max-width="400">
            <v-card>
                <v-card-title>{{
                    t('share.clearAllConfirm.title')
                }}</v-card-title>
                <v-card-text>
                    {{
                        t('share.clearAllConfirm.message', {
                            count: shareStore.accessRequests.size,
                        })
                    }}
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn variant="text" @click="showClearAllDialog = false">
                        {{ t('common.cancel') }}
                    </v-btn>
                    <v-btn
                        color="error"
                        variant="flat"
                        @click="confirmClearAll"
                    >
                        {{ t('share.requests.clearAll') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

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
import type { ShareSettings, ShareTransferStatus } from '../types'
import { formatSpeed } from '../types/transfer'

function formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}
import {
    mdiArrowLeft,
    mdiCheck,
    mdiClose,
    mdiDelete,
    mdiDeleteSweep,
} from '@mdi/js'

const router = useRouter()
const { t } = useI18n()
const shareStore = useShareStore()
const transferStore = useTransferStore()

const showError = ref(false)
const errorMessage = ref('')
const showShareSettings = ref(false)
const showPinConfig = ref(false)
const removeDialog = ref(false)
const showClearAllDialog = ref(false)
const requestIdToRemove = ref<string>('')
const shareSettings = ref<ShareSettings>({
    pinEnabled: false,
    pin: '',
    autoAccept: false,
})

/** 每个访问请求的折叠/展开状态 */
const expandedRequests = ref<Set<string>>(new Set())

// 格式化时间
function formatTime(timestamp: number): string {
    return new Date(timestamp).toLocaleTimeString()
}

// 判断某个请求的下载记录列表是否展开
function isExpanded(requestId: string): boolean {
    return expandedRequests.value.has(requestId)
}

// 切换折叠/展开状态
function toggleExpand(requestId: string): void {
    const newSet = new Set(expandedRequests.value)
    if (newSet.has(requestId)) {
        newSet.delete(requestId)
    } else {
        newSet.add(requestId)
    }
    expandedRequests.value = newSet
}

// 获取上传状态颜色
function getUploadStatusColor(status: ShareTransferStatus): string {
    const colorMap: Record<ShareTransferStatus, string> = {
        idle: 'grey',
        transferring: 'primary',
        completed: 'success',
        failed: 'error',
        cancelled: 'warning',
    }
    return colorMap[status] || 'grey'
}

// 获取上传状态文本
function getUploadStatusText(status: ShareTransferStatus): string {
    const keyMap: Record<ShareTransferStatus, string> = {
        idle: 'share.transfer.idle',
        transferring: 'share.transfer.transferring',
        completed: 'share.transfer.completed',
        failed: 'share.transfer.failed',
        cancelled: 'share.transfer.cancelled',
    }
    return t(keyMap[status] || 'share.transfer.idle')
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

// 显示移除单个请求对话框
function showRemoveDialog(requestId: string) {
    requestIdToRemove.value = requestId
    removeDialog.value = true
}

// 确认移除单个请求
async function confirmRemove() {
    try {
        await shareStore.removeRequest(requestIdToRemove.value)
        removeDialog.value = false
        requestIdToRemove.value = ''
    } catch (error) {
        showError.value = true
        errorMessage.value = t('share.removeError', { error })
    }
}

// 确认移除全部请求
async function confirmClearAll() {
    try {
        await shareStore.clearRequests()
        showClearAllDialog.value = false
    } catch (error) {
        showError.value = true
        errorMessage.value = t('share.clearAllError', { error })
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

.upload-records-container {
    background: rgba(var(--v-theme-surface-variant), 0.05);
    border-radius: 4px;
    padding: 8px;
}

.upload-record-item {
    padding: 4px 8px;
}

.upload-record-item.has-divider {
    border-bottom: 1px solid rgba(var(--v-border-color), 0.08);
}

.expanded-records {
    max-height: 400px;
    overflow-y: auto;
}
</style>
