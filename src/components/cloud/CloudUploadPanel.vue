<template>
    <v-card class="cloud-upload-panel">
        <v-card-title class="d-flex align-center justify-space-between">
            <div class="d-flex align-center">
                <span>{{ t('cloudTransfer.uploadToCloud') }}</span>
                <v-chip
                    v-if="pendingUploadTasks.length > 0"
                    color="error"
                    size="small"
                    class="ml-2"
                >
                    {{ pendingUploadTasks.length }}
                </v-chip>
            </div>
        </v-card-title>
        <v-card-text>
            <!-- 无账号提示 -->
            <v-alert v-if="!accountId" type="info" variant="tonal">
                {{ t('cloudTransfer.noAccounts') }}
                <template #append>
                    <v-btn size="small" variant="text" to="/settings">
                        {{ t('settings.title') }}
                    </v-btn>
                </template>
            </v-alert>

            <template v-else>
                <!-- 文件选择器 -->
                <FileSelector @select="handleFileSelect" class="mb-4" />

                <!-- 已选文件列表 -->
                <SelectedFileList
                    v-if="selectedFiles.count.value > 0"
                    :files="[...selectedFiles.files.value]"
                    :stats="selectedFiles.stats.value"
                    class="mb-4"
                    @remove="handleFileRemove"
                    @clear="handleFileClear"
                />

                <!-- 目标目录配置 -->
                <div v-if="selectedFiles.count.value > 0" class="mb-4">
                    <v-label>{{ t('cloudTransfer.targetDirectory') }}</v-label>
                    <div class="directory-input-wrapper mt-2">
                        <v-text-field
                            v-model="targetDirectory"
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

                    <!-- 上传设置 -->
                    <div class="upload-settings mt-4">
                        <div class="text-subtitle-2 mb-2">
                            {{ t('cloudTransfer.uploadSettings') }}
                        </div>
                        <v-switch
                            v-model="overwriteFiles"
                            :label="t('cloudTransfer.overwriteFiles')"
                            color="primary"
                            density="compact"
                            hide-details
                        />
                        <div class="text-caption text-grey">
                            {{
                                overwriteFiles
                                    ? t('cloudTransfer.overwriteHint')
                                    : t('cloudTransfer.renameHint')
                            }}
                        </div>
                    </div>

                    <!-- 上传按钮 -->
                    <div class="action-buttons mt-4">
                        <v-spacer />
                        <v-btn
                            color="primary"
                            variant="flat"
                            :prepend-icon="mdiCloudUpload"
                            :disabled="!canUpload"
                            :loading="uploading"
                            @click="handleUpload"
                        >
                            {{ t('cloudTransfer.uploadSelected') }}
                            <template #append>
                                <v-chip size="small" variant="text">
                                    {{ selectedFiles.count.value }}
                                </v-chip>
                            </template>
                        </v-btn>
                    </div>
                </div>

                <!-- 上传任务列表 -->
                <v-divider v-if="uploadTasks.length > 0" class="mb-4" />

                <div v-if="uploadTasks.length > 0">
                    <div class="text-subtitle-1 font-weight-medium mb-2">
                        {{ t('cloudTransfer.uploadTasks') }}
                    </div>
                    <v-list>
                        <template
                            v-for="(task, index) in visibleUploadTasks"
                            :key="task.id"
                        >
                            <v-list-item>
                                <v-list-item-title>
                                    {{ getAccountName(task.accountId) }}
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
                                <template #append>
                                    <v-btn
                                        :icon="mdiDelete"
                                        size="small"
                                        variant="text"
                                        color="grey"
                                        class="ml-2"
                                        @click="removeTask(task.id)"
                                    />
                                </template>
                            </v-list-item>

                            <!-- 文件上传详情列表 -->
                            <div
                                v-if="task.files.length > 0"
                                class="upload-records-container ml-4 mr-4 mb-2"
                            >
                                <!-- 折叠状态：显示前 3 条 -->
                                <template v-if="!isTaskExpanded(task.id)">
                                    <div
                                        v-for="(
                                            file, fileIndex
                                        ) in task.files.slice(0, 3)"
                                        :key="file.localPath"
                                        :class="[
                                            'upload-record-item',
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
                                        :key="file.localPath"
                                        :class="[
                                            'upload-record-item',
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
                                v-if="index < visibleUploadTasks.length - 1"
                            />
                        </template>
                    </v-list>

                    <!-- 展开/收起任务列表按钮 -->
                    <div v-if="uploadTasks.length > 3" class="text-center mt-2">
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
                            ({{ uploadTasks.length }})
                        </v-btn>
                    </div>
                </div>

                <!-- 空状态 -->
                <div
                    v-else-if="selectedFiles.count.value === 0"
                    class="d-flex flex-column align-center justify-center py-8"
                >
                    <v-icon
                        :icon="mdiCloudUpload"
                        size="64"
                        color="grey"
                        class="mb-4"
                    />
                    <div class="text-h6 text-grey">
                        {{ t('cloudTransfer.noUploadTasks') }}
                    </div>
                    <div class="text-body-2 text-grey">
                        {{ t('cloudTransfer.selectFileToUpload') }}
                    </div>
                </div>
            </template>
        </v-card-text>

        <!-- 目录浏览器对话框 -->
        <CloudDirectoryBrowser
            v-model="showDirectoryBrowser"
            :account-id="accountId"
            :initial-path="targetDirectory"
            mode="directory"
            @select="handleDirectorySelect"
        />
    </v-card>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { useCloudStore } from '@/stores/cloud'
import { useSelectedFiles } from '@/composables'
import { formatFileSize, formatTime, getFileStatusColor } from '@/utils/format'
import { listen } from '@tauri-apps/api/event'
import CloudDirectoryBrowser from './CloudDirectoryBrowser.vue'
import { FileSelector, SelectedFileList } from '@/components/transfer'
import type { UnlistenFn } from '@tauri-apps/api/event'
import type { CloudUploadTask } from '@/stores/cloud'
import { mdiFolder, mdiFolderOpen, mdiCloudUpload, mdiDelete } from '@mdi/js'

interface Props {
    accountId: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const cloudStore = useCloudStore()
const selectedFiles = useSelectedFiles()

const targetDirectory = ref<string>('/')
const showDirectoryBrowser = ref<boolean>(false)
const uploading = ref<boolean>(false)
const overwriteFiles = ref<boolean>(false)

// 上传任务列表 - 从 cloud store 获取
const taskListExpanded = ref(false)
const expandedTasks = reactive(new Set<string>())

// 使用 computed 从 store 获取任务列表
const uploadTasks = computed(() => cloudStore.uploadTasks)

let unlistenUploadProgress: UnlistenFn | null = null

const canUpload = computed(() => {
    return (
        selectedFiles.count.value > 0 &&
        targetDirectory.value.trim() !== '' &&
        !uploading.value
    )
})

const pendingUploadTasks = computed(() =>
    uploadTasks.value.filter((t) => t.status === 'uploading')
)

const visibleUploadTasks = computed(() => {
    const list = uploadTasks.value
    if (taskListExpanded.value || list.length <= 3) {
        return list
    }
    return list.slice(0, 3)
})

onMounted(async () => {
    // 初始化 cloud store（加载任务列表）
    await cloudStore.initialize()

    if (props.accountId) {
        const account = cloudStore.getAccountById(props.accountId)
        if (account && account.defaultDirectory) {
            targetDirectory.value = account.defaultDirectory
        } else {
            targetDirectory.value = '/'
        }
    }
    setupUploadProgressListener()
})

onUnmounted(() => {
    if (unlistenUploadProgress) {
        unlistenUploadProgress()
    }
})

async function setupUploadProgressListener(): Promise<void> {
    try {
        unlistenUploadProgress = await listen<{
            accountId: string
            localPath: string
            remotePath: string
            uploadedBytes: number
            totalBytes: number
            progress: number
            status: 'uploading' | 'completed' | 'failed'
        }>('cloud-upload-progress', (event) => {
            const payload = event.payload
            cloudStore.setUploadProgress(payload)

            // 更新任务列表
            updateUploadTask(payload)
        })
    } catch (error) {
        console.error('[CloudUploadPanel] 设置上传进度监听失败:', error)
    }
}

function updateUploadTask(payload: {
    accountId: string
    localPath: string
    remotePath: string
    uploadedBytes: number
    totalBytes: number
    progress: number
    status: 'uploading' | 'completed' | 'failed'
}): void {
    const taskId = `${payload.accountId}-${payload.remotePath}`
    const existingTask = uploadTasks.value.find((t) => t.id === taskId)

    let task: CloudUploadTask

    if (!existingTask) {
        // 创建新任务
        task = {
            id: taskId,
            accountId: payload.accountId,
            targetDirectory:
                payload.remotePath.substring(
                    0,
                    payload.remotePath.lastIndexOf('/')
                ) || '/',
            files: [],
            fileCount: 0,
            totalSize: 0,
            uploadedBytes: 0,
            progress: 0,
            status: 'uploading',
            createdAt: Date.now(),
        }
    } else {
        // 复制现有任务
        task = { ...existingTask, files: [...existingTask.files] }
    }

    // 查找或创建文件项
    let fileItem = task.files.find((f) => f.localPath === payload.localPath)
    if (!fileItem) {
        fileItem = {
            name: payload.localPath.split('/').pop() || payload.localPath,
            localPath: payload.localPath,
            remotePath: payload.remotePath,
            size: payload.totalBytes,
            uploadedBytes: payload.uploadedBytes,
            progress: payload.progress,
            status: payload.status,
            startedAt: Date.now(),
        }
        task.files.push(fileItem)
        task.fileCount = task.files.length
        task.totalSize = task.files.reduce((sum, f) => sum + f.size, 0)
    } else {
        fileItem.uploadedBytes = payload.uploadedBytes
        fileItem.progress = payload.progress
        fileItem.status = payload.status
    }

    // 更新任务统计
    task.uploadedBytes = task.files.reduce((sum, f) => sum + f.uploadedBytes, 0)
    task.progress =
        task.totalSize > 0
            ? Math.round((task.uploadedBytes / task.totalSize) * 100)
            : 0

    // 更新任务状态
    const allCompleted = task.files.every((f) => f.status === 'completed')
    const hasFailed = task.files.some((f) => f.status === 'failed')
    const isUploading = task.files.some((f) => f.status === 'uploading')

    if (allCompleted) {
        task.status = 'completed'
        task.completedAt = Date.now()
    } else if (hasFailed && !isUploading) {
        task.status = 'failed'
        task.completedAt = Date.now()
    } else {
        task.status = 'uploading'
    }

    // 使用 store 方法保存任务（会自动同步到历史记录）
    cloudStore.setUploadTask(task)
}

function getAccountName(accountId: string): string {
    const account = cloudStore.getAccountById(accountId)
    return account?.name || accountId
}

function getFileStatusText(status: string): string {
    const keyMap: Record<string, string> = {
        pending: 'cloudTransfer.fileStatus.pending',
        uploading: 'cloudTransfer.fileStatus.uploading',
        completed: 'cloudTransfer.fileStatus.completed',
        failed: 'cloudTransfer.fileStatus.failed',
    }
    return t(keyMap[status] || 'cloudTransfer.fileStatus.pending')
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

function removeTask(taskId: string) {
    cloudStore.removeUploadTask(taskId)
}

function handleFileSelect(file: {
    path: string
    name: string
    size: number
    type: string
}): void {
    selectedFiles.addFile({
        path: file.path,
        name: file.name,
        size: file.size,
        mimeType: file.type,
        sourceType: 'file',
    })
}

function handleFileRemove(path: string): void {
    selectedFiles.removeFile(path)
}

function handleFileClear(): void {
    selectedFiles.clearFiles()
}

function openDirectoryBrowser(): void {
    showDirectoryBrowser.value = true
}

function handleDirectorySelect(path: string): void {
    targetDirectory.value = path
}

async function handleUpload(): Promise<void> {
    if (!props.accountId || selectedFiles.count.value === 0) return

    uploading.value = true
    try {
        const filesToUpload = [...selectedFiles.files.value]
        for (const file of filesToUpload) {
            const remotePath =
                targetDirectory.value === '/'
                    ? `/${file.name}`
                    : `${targetDirectory.value}/${file.name}`

            await cloudStore.uploadFile(
                props.accountId,
                file.path,
                remotePath,
                overwriteFiles.value
            )
        }
        selectedFiles.clearFiles()
    } catch (error) {
        console.error('[CloudUploadPanel] 上传文件失败:', error)
    } finally {
        uploading.value = false
    }
}
</script>

<style scoped>
.cloud-upload-panel {
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

.upload-settings {
    padding: 12px 16px;
    background-color: rgba(var(--v-theme-surface-variant), 0.1);
    border-radius: 8px;
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
    max-height: 300px;
    overflow-y: auto;
}
</style>
