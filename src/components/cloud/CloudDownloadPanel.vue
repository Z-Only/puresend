<template>
    <v-card class="cloud-download-panel">
        <v-card-title class="d-flex align-center justify-space-between">
            <div class="d-flex align-center">
                <span>{{ t('cloudTransfer.downloadFromCloud') }}</span>
                <v-chip
                    v-if="pendingDownloadTasks.length > 0"
                    color="error"
                    size="small"
                    class="ml-2"
                >
                    {{ pendingDownloadTasks.length }}
                </v-chip>
            </div>
        </v-card-title>
        <v-card-text>
            <!-- 无账号提示 -->
            <v-alert v-if="!accountId" type="info" variant="tonal">
                {{ t('cloudTransfer.noAccountSelected') }}
            </v-alert>

            <template v-else>
                <!-- 下载目录配置 -->
                <div class="mb-4">
                    <v-label>{{
                        t('cloudTransfer.downloadDirectory')
                    }}</v-label>
                    <div class="directory-input-wrapper mt-2">
                        <v-text-field
                            v-model="downloadDirectory"
                            variant="outlined"
                            density="compact"
                            :prepend-inner-icon="mdiFolderOutline"
                            readonly
                        />
                        <v-btn
                            color="primary"
                            variant="outlined"
                            :prepend-icon="mdiFolderOpen"
                            @click="selectDownloadDirectory"
                        >
                            {{ t('common.browse') }}
                        </v-btn>
                    </div>
                </div>

                <!-- 文件浏览器和选择区域 -->
                <div class="mb-4">
                    <!-- 操作按钮 -->
                    <div class="action-buttons mb-4">
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

                    <!-- 目录面包屑导航 -->
                    <div class="directory-breadcrumb">
                        <v-breadcrumbs :items="breadcrumbItems">
                            <template #divider>
                                <v-icon :icon="mdiChevronRight" />
                            </template>
                            <template #item="{ item }">
                                <v-breadcrumbs-item
                                    :disabled="item.disabled"
                                    @click="
                                        navigateToPath(
                                            (item as BreadcrumbItem).path
                                        )
                                    "
                                >
                                    <v-icon
                                        v-if="(item as BreadcrumbItem).isRoot"
                                        :icon="mdiFolder"
                                        class="mr-1"
                                    />
                                    {{ item.title }}
                                </v-breadcrumbs-item>
                            </template>
                        </v-breadcrumbs>
                    </div>

                    <!-- 远程文件列表 -->
                    <v-list
                        v-if="remoteFiles.length > 0"
                        class="file-list custom-scrollbar"
                    >
                        <v-list-item
                            v-for="file in remoteFiles"
                            :key="file.path"
                            :value="file.path"
                            density="compact"
                            @click="handleFileClick(file)"
                        >
                            <template #prepend>
                                <v-icon
                                    :icon="
                                        getFileTypeIcon(
                                            file.name,
                                            file.isDirectory || false
                                        )
                                    "
                                    :color="
                                        file.isDirectory ? 'warning' : 'info'
                                    "
                                    size="24"
                                    class="mr-2"
                                />
                            </template>
                            <template #title>
                                <div class="file-title-row">
                                    <span class="file-name text-truncate">{{
                                        file.name
                                    }}</span>
                                    <span class="file-meta">
                                        <span class="file-size">
                                            {{
                                                formatFileSizeSafe(
                                                    file.size,
                                                    file.isDirectory || false
                                                )
                                            }}
                                        </span>
                                        <span class="file-time">
                                            {{
                                                file.modified
                                                    ? formatTime(file.modified)
                                                    : ''
                                            }}
                                        </span>
                                    </span>
                                </div>
                            </template>
                            <template #append>
                                <v-checkbox-btn
                                    v-if="!file.isDirectory"
                                    v-model="selectedFiles"
                                    :value="file.path"
                                    color="primary"
                                    @click.stop
                                />
                                <v-checkbox-btn
                                    v-else
                                    :model-value="false"
                                    disabled
                                    class="invisible-checkbox"
                                />
                            </template>
                        </v-list-item>
                    </v-list>
                    <v-alert
                        v-else-if="!loadingFiles"
                        type="info"
                        variant="tonal"
                        class="mt-4"
                    >
                        {{ t('cloudTransfer.noFilesSelected') }}
                    </v-alert>
                    <v-skeleton-loader v-else type="list-item@3" class="mt-4" />
                </div>

                <!-- 下载任务列表 -->
                <v-divider v-if="downloadTasks.length > 0" class="mb-4" />

                <div v-if="downloadTasks.length > 0">
                    <div class="text-subtitle-1 font-weight-medium mb-2">
                        {{ t('cloudTransfer.downloadTasks') }}
                    </div>
                    <v-list>
                        <template
                            v-for="(task, index) in visibleDownloadTasks"
                            :key="task.id"
                        >
                            <v-list-item>
                                <v-list-item-title>
                                    {{ task.accountName }}
                                </v-list-item-title>
                                <v-list-item-subtitle>
                                    {{ formatTime(task.createdAt) }}
                                    <template v-if="task.fileCount > 0">
                                        ·
                                        {{
                                            t('cloudTransfer.fileCount', {
                                                count: task.fileCount,
                                            })
                                        }}
                                        ·
                                        {{ formatFileSize(task.totalSize) }}
                                    </template>
                                </v-list-item-subtitle>
                            </v-list-item>

                            <!-- 文件下载详情列表 -->
                            <div
                                v-if="task.files.length > 0"
                                class="download-records-container ml-4 mr-4 mb-2"
                            >
                                <!-- 折叠状态：显示前 3 条 -->
                                <template v-if="!isTaskExpanded(task.id)">
                                    <div
                                        v-for="(
                                            file, fileIndex
                                        ) in task.files.slice(0, 3)"
                                        :key="file.name"
                                        :class="[
                                            'download-record-item',
                                            {
                                                'has-divider':
                                                    fileIndex <
                                                    task.files.slice(0, 3)
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
                                                    {{ file.name }}
                                                </span>
                                                <span
                                                    v-if="file.startedAt"
                                                    class="text-caption text-grey ml-2"
                                                    style="white-space: nowrap"
                                                >
                                                    {{
                                                        formatTime(
                                                            file.startedAt
                                                        )
                                                    }}
                                                </span>
                                                <span
                                                    class="text-caption text-grey ml-2"
                                                    style="white-space: nowrap"
                                                >
                                                    {{
                                                        formatFileSize(
                                                            file.size
                                                        )
                                                    }}
                                                </span>
                                            </div>
                                            <div
                                                class="d-flex align-center ga-2"
                                            >
                                                <span
                                                    v-if="
                                                        file.status ===
                                                        'downloading'
                                                    "
                                                    class="text-body-2 text-grey"
                                                >
                                                    {{
                                                        formatSpeed(file.speed)
                                                    }}
                                                </span>
                                                <span class="text-body-2">
                                                    {{
                                                        file.progress.toFixed(
                                                            1
                                                        )
                                                    }}%
                                                </span>
                                                <v-chip
                                                    :color="
                                                        getFileStatusColor(
                                                            file.status
                                                        )
                                                    "
                                                    size="x-small"
                                                    label
                                                >
                                                    {{
                                                        getFileStatusText(
                                                            file.status
                                                        )
                                                    }}
                                                </v-chip>
                                            </div>
                                        </div>
                                        <v-progress-linear
                                            v-if="file.status !== 'pending'"
                                            :model-value="file.progress"
                                            :color="
                                                file.status === 'completed'
                                                    ? 'success'
                                                    : file.status === 'failed'
                                                      ? 'error'
                                                      : 'primary'
                                            "
                                            height="3"
                                            class="mt-1"
                                        />
                                    </div>
                                </template>

                                <!-- 展开状态：显示最多10条文件（可滚动） -->
                                <div
                                    v-show="isTaskExpanded(task.id)"
                                    class="expanded-records custom-scrollbar"
                                >
                                    <div
                                        v-for="(
                                            file, fileIndex
                                        ) in task.files.slice(0, 10)"
                                        :key="file.name"
                                        :class="[
                                            'download-record-item',
                                            {
                                                'has-divider':
                                                    fileIndex <
                                                    task.files.slice(0, 10)
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
                                                    {{ file.name }}
                                                </span>
                                                <span
                                                    v-if="file.startedAt"
                                                    class="text-caption text-grey ml-2"
                                                    style="white-space: nowrap"
                                                >
                                                    {{
                                                        formatTime(
                                                            file.startedAt
                                                        )
                                                    }}
                                                </span>
                                                <span
                                                    class="text-caption text-grey ml-2"
                                                    style="white-space: nowrap"
                                                >
                                                    {{
                                                        formatFileSize(
                                                            file.size
                                                        )
                                                    }}
                                                </span>
                                            </div>
                                            <div
                                                class="d-flex align-center ga-2"
                                            >
                                                <span
                                                    v-if="
                                                        file.status ===
                                                        'downloading'
                                                    "
                                                    class="text-body-2 text-grey"
                                                >
                                                    {{
                                                        formatSpeed(file.speed)
                                                    }}
                                                </span>
                                                <span class="text-body-2">
                                                    {{
                                                        file.progress.toFixed(
                                                            1
                                                        )
                                                    }}%
                                                </span>
                                                <v-chip
                                                    :color="
                                                        getFileStatusColor(
                                                            file.status
                                                        )
                                                    "
                                                    size="x-small"
                                                    label
                                                >
                                                    {{
                                                        getFileStatusText(
                                                            file.status
                                                        )
                                                    }}
                                                </v-chip>
                                            </div>
                                        </div>
                                        <v-progress-linear
                                            v-if="file.status !== 'pending'"
                                            :model-value="file.progress"
                                            :color="
                                                file.status === 'completed'
                                                    ? 'success'
                                                    : file.status === 'failed'
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
                                    v-if="task.files.length > 3"
                                    class="text-center mt-1"
                                >
                                    <v-btn
                                        variant="text"
                                        size="small"
                                        density="compact"
                                        @click="toggleFileList(task.id)"
                                    >
                                        <template
                                            v-if="!isTaskExpanded(task.id)"
                                        >
                                            {{
                                                t('cloudTransfer.moreFiles', {
                                                    count:
                                                        task.files.length - 3,
                                                })
                                            }}
                                        </template>
                                        <template v-else>
                                            {{
                                                t('cloudTransfer.collapseFiles')
                                            }}
                                        </template>
                                    </v-btn>
                                </div>
                            </div>

                            <!-- 错误信息 -->
                            <v-alert
                                v-if="task.error"
                                type="error"
                                variant="tonal"
                                class="mx-4 mb-2"
                                density="compact"
                            >
                                {{ task.error }}
                            </v-alert>

                            <!-- 只在不是最后一条任务时显示分隔线 -->
                            <v-divider
                                v-if="index < visibleDownloadTasks.length - 1"
                            />
                        </template>
                    </v-list>

                    <!-- 展开/收起任务列表按钮 -->
                    <div
                        v-if="downloadTasks.length > 3"
                        class="text-center mt-2"
                    >
                        <v-btn
                            variant="text"
                            size="small"
                            @click="taskListExpanded = !taskListExpanded"
                        >
                            {{
                                taskListExpanded
                                    ? t('cloudTransfer.collapse')
                                    : t('cloudTransfer.expand')
                            }}
                            ({{ downloadTasks.length }})
                        </v-btn>
                    </div>
                </div>

                <!-- 空状态 -->
                <div
                    v-else-if="selectedFiles.length === 0"
                    class="d-flex flex-column align-center justify-center py-8"
                >
                    <v-icon
                        :icon="mdiInboxArrowDown"
                        size="64"
                        color="grey"
                        class="mb-4"
                    />
                    <div class="text-h6 text-grey">
                        {{ t('cloudTransfer.noDownloadTasks') }}
                    </div>
                    <div class="text-body-2 text-grey">
                        {{ t('cloudTransfer.browseFilesToStart') }}
                    </div>
                </div>
            </template>
        </v-card-text>
    </v-card>
</template>

<script setup lang="ts">
import { ref, computed, reactive, onMounted, onUnmounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useCloudStore } from '@/stores/cloud'
import { useTransferStore } from '@/stores'
import { open } from '@tauri-apps/plugin-dialog'
import { listen } from '@tauri-apps/api/event'
import { getReceiveDirectory } from '@/services/transferService'
import {
    formatFileSize,
    formatFileSizeSafe,
    formatSpeed,
    formatTime,
    getFileTypeIcon,
    getFileStatusColor,
} from '@/utils/format'
import type { CloudFileItem } from '@/types/cloud'
import type { UnlistenFn } from '@tauri-apps/api/event'
import type { CloudDownloadTask } from '@/stores/cloud'
import {
    mdiFolder,
    mdiFolderOutline,
    mdiFolderOpen,
    mdiChevronRight,
    mdiRefresh,
    mdiDownload,
    mdiInboxArrowDown,
} from '@mdi/js'

interface BreadcrumbItem {
    title: string
    path: string
    disabled: boolean
    isRoot: boolean
}

interface Props {
    accountId: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const cloudStore = useCloudStore()
const transferStore = useTransferStore()

const sourceDirectory = ref<string>('/')
const remoteFiles = ref<CloudFileItem[]>([])
const selectedFiles = ref<string[]>([])
const loadingFiles = ref<boolean>(false)
const downloading = ref<boolean>(false)
const downloadDirectory = ref<string>('')
const taskListExpanded = ref(false)
const expandedTasks = reactive(new Set<string>())

// 使用 computed 从 store 获取任务列表
const downloadTasks = computed(() => cloudStore.downloadTasks)

let unlistenDownloadProgress: UnlistenFn | null = null

const breadcrumbItems = computed<BreadcrumbItem[]>(() => {
    if (!sourceDirectory.value) return []

    const parts = sourceDirectory.value.split('/').filter(Boolean)
    const items: BreadcrumbItem[] = [
        {
            title: t('cloudTransfer.directoryBrowser.currentPath'),
            path: '/',
            disabled: sourceDirectory.value === '/',
            isRoot: true,
        },
    ]

    let currentPath = ''
    parts.forEach((part, index) => {
        currentPath += `/${part}`
        items.push({
            title: part,
            path: currentPath,
            disabled: index === parts.length - 1,
            isRoot: false,
        })
    })

    return items
})

const visibleDownloadTasks = computed(() => {
    const list = downloadTasks.value
    if (taskListExpanded.value || list.length <= 3) {
        return list
    }
    return list.slice(0, 3)
})

const pendingDownloadTasks = computed(() => {
    return downloadTasks.value.filter(
        (task) => task.transferStatus === 'downloading'
    )
})

onMounted(async () => {
    // 初始化 cloud store（加载任务列表）
    await cloudStore.initialize()
    // 获取正确的接收目录（绝对路径）
    await initializeDownloadDirectory()
    await initializePanel()
    setupDownloadProgressListener()
})

onUnmounted(() => {
    if (unlistenDownloadProgress) {
        unlistenDownloadProgress()
    }
})

watch(
    () => props.accountId,
    async (newAccountId) => {
        if (newAccountId) {
            await initializePanel()
        }
    }
)

/**
 * 初始化下载目录 - 从后端获取绝对路径
 */
async function initializeDownloadDirectory(): Promise<void> {
    try {
        // 优先从后端获取绝对路径
        const dir = await getReceiveDirectory()
        downloadDirectory.value = dir
    } catch (error) {
        console.error('[CloudDownloadPanel] 获取接收目录失败:', error)
        // 降级使用 store 中的值（可能是带 ~ 的路径）
        downloadDirectory.value = transferStore.receiveDirectory
    }
}

async function initializePanel(): Promise<void> {
    if (!props.accountId) return

    // 默认目录为云盘根目录
    sourceDirectory.value = '/'
    selectedFiles.value = []
    await refreshFiles()
}

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
            updateDownloadTask(event.payload)
        })
    } catch (error) {
        console.error('[CloudDownloadPanel] 设置下载进度监听失败:', error)
    }
}

function updateDownloadTask(progress: {
    accountId: string
    remotePath: string
    localPath: string
    downloadedBytes: number
    totalBytes: number
    progress: number
    status: 'downloading' | 'completed' | 'failed'
}): void {
    const account = cloudStore.getAccountById(progress.accountId)
    const accountName = account?.name || progress.accountId

    const existingTask = downloadTasks.value.find(
        (t) => t.accountId === progress.accountId
    )

    let task: CloudDownloadTask

    if (!existingTask) {
        task = {
            id: `download-${progress.accountId}-${Date.now()}`,
            accountId: progress.accountId,
            accountName,
            files: [],
            fileCount: 0,
            totalSize: 0,
            totalTransferredBytes: 0,
            transferStatus: progress.status,
            progress: progress.progress,
            speed: 0,
            createdAt: Date.now(),
        }
    } else {
        // 复制现有任务
        task = { ...existingTask, files: [...existingTask.files] }
    }

    const fileName = progress.remotePath.split('/').pop() || progress.remotePath
    let file = task.files.find((f) => f.name === fileName)

    if (!file) {
        file = {
            name: fileName,
            size: progress.totalBytes,
            transferredBytes: progress.downloadedBytes,
            progress: progress.progress,
            speed: 0,
            status:
                progress.status === 'downloading'
                    ? 'downloading'
                    : progress.status,
            startedAt: Date.now(),
        }
        task.files.unshift(file)
        task.fileCount = task.files.length
        task.totalSize = task.files.reduce((sum, f) => sum + f.size, 0)
    } else {
        file.transferredBytes = progress.downloadedBytes
        file.progress = progress.progress
        file.status =
            progress.status === 'downloading' ? 'downloading' : progress.status
    }

    task.transferStatus = progress.status
    task.totalTransferredBytes = task.files.reduce(
        (sum, f) => sum + f.transferredBytes,
        0
    )
    task.progress =
        task.totalSize > 0
            ? (task.totalTransferredBytes / task.totalSize) * 100
            : 0

    if (progress.status === 'completed' || progress.status === 'failed') {
        downloading.value = false
        task.completedAt = Date.now()
    }

    // 使用 store 方法保存任务（会自动同步到历史记录）
    cloudStore.setDownloadTask(task)
}

async function selectDownloadDirectory(): Promise<void> {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            title: t('cloudTransfer.selectDownloadDirectory'),
        })
        if (selected && typeof selected === 'string') {
            downloadDirectory.value = selected
        }
    } catch (error) {
        console.error('[CloudDownloadPanel] 选择下载目录失败:', error)
    }
}

function handleFileClick(file: CloudFileItem): void {
    if (file.isDirectory) {
        sourceDirectory.value = file.path
        selectedFiles.value = []
        refreshFiles()
    }
}

function navigateToPath(path: string): void {
    if (path !== sourceDirectory.value) {
        sourceDirectory.value = path
        selectedFiles.value = []
        refreshFiles()
    }
}

async function refreshFiles(): Promise<void> {
    if (!props.accountId) return

    loadingFiles.value = true
    try {
        remoteFiles.value = await cloudStore.browseDirectory(
            props.accountId,
            sourceDirectory.value
        )
    } catch (error) {
        console.error('[CloudDownloadPanel] 刷新文件列表失败:', error)
        remoteFiles.value = []
    } finally {
        loadingFiles.value = false
    }
}

async function downloadSelectedFiles(): Promise<void> {
    if (selectedFiles.value.length === 0 || !props.accountId) return

    downloading.value = true
    try {
        for (const remotePath of selectedFiles.value) {
            const fileName = remotePath.split('/').pop() || remotePath
            const localPath = `${downloadDirectory.value}/${fileName}`

            await cloudStore.downloadFile(
                props.accountId,
                remotePath,
                localPath
            )
        }
        selectedFiles.value = []
    } catch (error) {
        console.error('[CloudDownloadPanel] 下载文件失败:', error)
    } finally {
        downloading.value = false
    }
}

function isTaskExpanded(taskId: string): boolean {
    return expandedTasks.has(taskId)
}

function toggleFileList(taskId: string) {
    if (expandedTasks.has(taskId)) {
        expandedTasks.delete(taskId)
    } else {
        expandedTasks.add(taskId)
    }
}

function getFileStatusText(status: string): string {
    const keyMap: Record<string, string> = {
        pending: 'cloudTransfer.fileStatus.pending',
        downloading: 'cloudTransfer.fileStatus.downloading',
        completed: 'cloudTransfer.fileStatus.completed',
        failed: 'cloudTransfer.fileStatus.failed',
    }
    return t(keyMap[status] || 'cloudTransfer.fileStatus.pending')
}
</script>

<style scoped>
.cloud-download-panel {
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

.directory-breadcrumb {
    padding: 8px 16px;
    background-color: rgba(var(--v-theme-surface-variant), 0.1);
}

.directory-breadcrumb :deep(.v-breadcrumbs) {
    padding: 0;
}

.directory-breadcrumb
    :deep(.v-breadcrumbs-item:not(.v-breadcrumbs-item--disabled)) {
    cursor: pointer;
    color: rgb(var(--v-theme-primary));
}

.directory-breadcrumb
    :deep(.v-breadcrumbs-item:not(.v-breadcrumbs-item--disabled)):hover {
    text-decoration: underline;
}

.directory-breadcrumb :deep(.v-breadcrumbs-item--disabled) {
    cursor: default;
    opacity: 1;
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

.file-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
}

.file-name {
    font-weight: 500;
    flex: 1;
    min-width: 0;
}

.file-meta {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
}

.file-size {
    display: inline-block;
    min-width: 80px;
    text-align: left;
    color: rgba(var(--v-theme-on-surface), 0.6);
    font-size: 0.875rem;
    white-space: nowrap;
}

.file-time {
    display: inline-block;
    min-width: 160px;
    text-align: left;
    color: rgba(var(--v-theme-on-surface), 0.6);
    font-size: 0.875rem;
    white-space: nowrap;
}

.invisible-checkbox {
    visibility: hidden;
}

.download-records-container {
    background: rgba(var(--v-theme-surface-variant), 0.05);
    border-radius: 4px;
    padding: 8px;
}

.download-record-item {
    padding: 4px 8px;
}

.download-record-item.has-divider {
    border-bottom: 1px solid rgba(var(--v-border-color), 0.08);
}

.expanded-records {
    max-height: 300px;
    overflow-y: auto;
}
</style>
