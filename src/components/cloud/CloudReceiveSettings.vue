<template>
    <v-card class="cloud-receive-settings">
        <v-card-text>
            <!-- 账号选择器 -->
            <v-select
                v-model="selectedAccountId"
                :label="t('cloudTransfer.selectAccount')"
                :items="cloudStore.accounts"
                item-title="name"
                item-value="id"
                :disabled="!cloudStore.hasAccounts"
                variant="outlined"
                :prepend-inner-icon="mdiCloud"
                @update:model-value="handleAccountChange"
            >
                <template #item="{ item, props }">
                    <v-list-item v-bind="props">
                        <template #prepend>
                            <v-icon :icon="mdiCloud" />
                        </template>
                        <template #append>
                            <v-chip
                                :color="getStatusColor(item.status)"
                                size="small"
                                variant="flat"
                            >
                                {{ getStatusText(item.status) }}
                            </v-chip>
                        </template>
                    </v-list-item>
                </template>
            </v-select>

            <!-- 无账号提示 -->
            <v-alert
                v-if="!cloudStore.hasAccounts"
                type="info"
                variant="tonal"
                class="mt-4"
            >
                {{ t('cloudTransfer.noAccounts') }}
                <template #append>
                    <v-btn size="small" variant="text" to="/settings">
                        {{ t('settings.title') }}
                    </v-btn>
                </template>
            </v-alert>

            <!-- 中转目录配置和文件列表 -->
            <div v-if="selectedAccountId" class="mt-4">
                <v-label>{{ t('cloudTransfer.sourceDirectory') }}</v-label>
                <div class="directory-input-wrapper mt-2">
                    <v-text-field
                        v-model="sourceDirectory"
                        variant="outlined"
                        density="compact"
                        :prepend-inner-icon="mdiFolder"
                        readonly
                    />
                    <v-btn
                        color="primary"
                        variant="outlined"
                        :prepend-icon="mdiFolderOpen"
                        @click="openDirectoryBrowser"
                    >
                        {{ t('cloudTransfer.browse') }}
                    </v-btn>
                </div>

                <!-- 操作按钮 -->
                <div class="action-buttons mt-4">
                    <v-btn
                        variant="outlined"
                        :prepend-icon="mdiRefresh"
                        :loading="loadingFiles"
                        @click="refreshFiles"
                    >
                        {{ t('cloudTransfer.refreshFiles') }}
                    </v-btn>
                    <v-spacer />
                    <v-btn
                        color="primary"
                        variant="flat"
                        :prepend-icon="mdiDownload"
                        :disabled="selectedFiles.length === 0"
                        :loading="downloading"
                        @click="downloadSelectedFiles"
                    >
                        {{ t('cloudTransfer.downloadSelected') }}
                        <template #append>
                            <v-chip size="small" variant="text">
                                {{ selectedFiles.length }}
                            </v-chip>
                        </template>
                    </v-btn>
                </div>

                <!-- 远程文件列表 -->
                <v-card variant="outlined" class="mt-4">
                    <v-card-text class="pa-0">
                        <v-list v-if="remoteFiles.length > 0" class="file-list">
                            <v-list-item
                                v-for="file in remoteFiles"
                                :key="file.path"
                                :value="file.path"
                                density="compact"
                            >
                                <template #prepend>
                                    <v-icon
                                        :icon="mdiFile"
                                        :color="
                                            file.isDirectory
                                                ? 'warning'
                                                : 'info'
                                        "
                                        size="24"
                                        class="mr-2"
                                    />
                                </template>
                                <template #title>
                                    <div class="file-title-row">
                                        <span class="file-name">{{
                                            file.name
                                        }}</span>
                                        <span
                                            class="file-size"
                                            v-if="file.size !== undefined"
                                        >
                                            {{ formatFileSize(file.size) }}
                                        </span>
                                        <span
                                            v-if="file.modified"
                                            class="file-time"
                                        >
                                            {{ formatTime(file.modified) }}
                                        </span>
                                    </div>
                                </template>
                                <template #append>
                                    <v-checkbox-btn
                                        v-model="selectedFiles"
                                        :value="file.path"
                                        color="primary"
                                    />
                                </template>
                            </v-list-item>
                        </v-list>
                        <v-alert
                            v-else-if="!loadingFiles"
                            type="info"
                            variant="tonal"
                            class="ma-4"
                        >
                            {{ t('cloudTransfer.noFilesSelected') }}
                        </v-alert>
                        <v-skeleton-loader
                            v-else
                            type="list-item@3"
                            class="ma-2"
                        />
                    </v-card-text>
                </v-card>

                <!-- 下载进度 -->
                <v-card
                    v-if="cloudStore.downloadProgress"
                    variant="outlined"
                    class="mt-4"
                >
                    <v-card-text>
                        <div class="download-progress">
                            <div class="progress-header">
                                <span class="progress-title">
                                    {{ t('cloudTransfer.downloading') }}
                                </span>
                                <span class="progress-status">
                                    {{ getDownloadStatusText() }}
                                </span>
                            </div>
                            <v-progress-linear
                                :model-value="
                                    cloudStore.downloadProgress.progress
                                "
                                color="primary"
                                height="8"
                                rounded
                            />
                            <div class="progress-details mt-2">
                                <span>
                                    {{
                                        formatFileSize(
                                            cloudStore.downloadProgress
                                                .downloadedBytes
                                        )
                                    }}
                                    /
                                    {{
                                        formatFileSize(
                                            cloudStore.downloadProgress
                                                .totalBytes
                                        )
                                    }}
                                </span>
                                <span>
                                    {{ cloudStore.downloadProgress.progress }}%
                                </span>
                            </div>
                        </div>
                    </v-card-text>
                </v-card>
            </div>
        </v-card-text>

        <!-- 目录浏览器对话框 -->
        <CloudDirectoryBrowser
            v-model="showDirectoryBrowser"
            :account-id="selectedAccountId"
            :initial-path="sourceDirectory"
            mode="directory"
            @select="handleDirectorySelect"
        />
    </v-card>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useCloudStore } from '@/stores/cloud'
import { useTransferStore } from '@/stores'
import { formatFileSize } from '@/utils/format'
import { listen } from '@tauri-apps/api/event'
import CloudDirectoryBrowser from './CloudDirectoryBrowser.vue'
import type { CloudAccountStatus, CloudFileItem } from '@/types/cloud'
import type { UnlistenFn } from '@tauri-apps/api/event'
import {
    mdiFile,
    mdiCloud,
    mdiFolder,
    mdiFolderOpen,
    mdiRefresh,
    mdiDownload,
} from '@mdi/js'

const { t } = useI18n()
const cloudStore = useCloudStore()
const transferStore = useTransferStore()

const selectedAccountId = ref<string>('')
const sourceDirectory = ref<string>('')
const remoteFiles = ref<CloudFileItem[]>([])
const selectedFiles = ref<string[]>([])
const showDirectoryBrowser = ref<boolean>(false)
const loadingFiles = ref<boolean>(false)
const downloading = ref<boolean>(false)

let unlistenDownloadProgress: UnlistenFn | null = null

onMounted(async () => {
    await cloudStore.loadAccounts()
    if (cloudStore.hasAccounts) {
        selectedAccountId.value = cloudStore.accounts[0].id
        // 默认目录为云盘根目录
        sourceDirectory.value = '/'
        await refreshFiles()
    }
    setupDownloadProgressListener()
})

onUnmounted(() => {
    if (unlistenDownloadProgress) {
        unlistenDownloadProgress()
    }
})

async function setupDownloadProgressListener(): Promise<void> {
    try {
        unlistenDownloadProgress = await listen<{
            accountId: string
            remotePath: string
            localPath: string
            downloadedBytes: number
            totalBytes: number
            progress: number
            status: 'downloading' | 'completed' | 'failed'
        }>('cloud-download-progress', (event) => {
            cloudStore.setDownloadProgress(event.payload)
            if (
                event.payload.status === 'completed' ||
                event.payload.status === 'failed'
            ) {
                downloading.value = false
            }
        })
    } catch (error) {
        console.error('[CloudReceiveSettings] 设置下载进度监听失败:', error)
    }
}

function handleAccountChange(): void {
    // 切换账号时，默认目录为云盘根目录
    sourceDirectory.value = '/'
    selectedFiles.value = []
    refreshFiles()
}

function openDirectoryBrowser(): void {
    showDirectoryBrowser.value = true
}

function handleDirectorySelect(path: string): void {
    sourceDirectory.value = path
    selectedFiles.value = []
    refreshFiles()
}

async function refreshFiles(): Promise<void> {
    if (!selectedAccountId.value) return

    loadingFiles.value = true
    try {
        remoteFiles.value = await cloudStore.browseDirectory(
            selectedAccountId.value,
            sourceDirectory.value
        )
    } catch (error) {
        console.error('[CloudReceiveSettings] 刷新文件列表失败:', error)
        remoteFiles.value = []
    } finally {
        loadingFiles.value = false
    }
}

async function downloadSelectedFiles(): Promise<void> {
    if (selectedFiles.value.length === 0 || !selectedAccountId.value) return

    downloading.value = true
    try {
        for (const remotePath of selectedFiles.value) {
            const fileName = remotePath.split('/').pop() || remotePath
            const receiveDir = transferStore.receiveDirectory || '~/Downloads'
            const localPath = `${receiveDir}/${fileName}`

            await cloudStore.downloadFile(
                selectedAccountId.value,
                remotePath,
                localPath
            )
        }
    } catch (error) {
        console.error('[CloudReceiveSettings] 下载文件失败:', error)
    } finally {
        downloading.value = false
    }
}

function getStatusColor(status: CloudAccountStatus): string {
    const colorMap: Record<CloudAccountStatus, string> = {
        connected: 'success',
        disconnected: 'warning',
        invalid: 'error',
    }
    return colorMap[status] || 'grey'
}

function getStatusText(status: CloudAccountStatus): string {
    const textMap: Record<CloudAccountStatus, string> = {
        connected: t('common.enabled'),
        disconnected: t('common.disabled'),
        invalid: 'invalid',
    }
    return textMap[status] || status
}

function formatTime(timestamp: number): string {
    return new Date(timestamp).toLocaleString()
}

function getDownloadStatusText(): string {
    if (!cloudStore.downloadProgress) return ''
    const statusMap: Record<string, string> = {
        downloading: t('cloudTransfer.downloading'),
        completed: t('cloudTransfer.downloadSuccess'),
        failed: t('cloudTransfer.downloadFailed'),
    }
    return statusMap[cloudStore.downloadProgress.status] || ''
}
</script>

<style scoped>
.cloud-receive-settings {
    width: 100%;
}

.directory-input-wrapper {
    display: flex;
    gap: 8px;
    align-items: flex-start;
}

.directory-input-wrapper .v-text-field {
    flex: 1;
}

.action-buttons {
    display: flex;
    align-items: center;
    gap: 8px;
}

.file-list {
    max-height: 400px;
    overflow-y: auto;
}

.file-name {
    font-weight: 500;
    flex-shrink: 0;
}

.file-title-row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
}

.file-size {
    flex-shrink: 0;
    font-size: 0.875rem;
    color: rgba(0, 0, 0, 0.6);
    white-space: nowrap;
}

.file-time {
    font-size: 0.875rem;
    color: rgba(0, 0, 0, 0.6);
    white-space: nowrap;
}

.download-progress {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.progress-title {
    font-weight: 500;
}

.progress-status {
    font-size: 0.875rem;
    color: rgba(0, 0, 0, 0.6);
}

.progress-details {
    display: flex;
    justify-content: space-between;
    font-size: 0.875rem;
    color: rgba(0, 0, 0, 0.6);
}
</style>
