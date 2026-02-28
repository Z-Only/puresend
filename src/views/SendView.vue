<!-- 文件发送页面 -->
<template>
    <v-container fluid class="send-view">
        <v-row>
            <!-- 左侧：文件选择和传输设置 -->
            <v-col cols="12" md="6">
                <!-- 内容类型选择 -->
                <ContentTypeSelector @change="handleContentTypeChange" />

                <!-- 文件选择 -->
                <FileSelector
                    v-if="contentType === 'file'"
                    @select="handleFileSelect"
                />

                <!-- 文件夹选择 -->
                <FolderPicker
                    v-else-if="contentType === 'folder'"
                    @select="handleFolderSelect"
                />

                <!-- 剪贴板导入 -->
                <ClipboardImporter
                    v-else-if="contentType === 'clipboard'"
                    @select="handleClipboardSelect"
                />

                <!-- 文本输入 -->
                <TextInput
                    v-else-if="contentType === 'text'"
                    @select="handleTextSelect"
                />

                <!-- 媒体选择 -->
                <MediaPicker
                    v-else-if="contentType === 'media'"
                    @select="handleMediaSelect"
                />

                <!-- 应用选择 -->
                <AppPicker
                    v-else-if="contentType === 'app'"
                    @select="handleAppSelect"
                />

                <!-- 已选文件列表 -->
                <SelectedFileList
                    v-if="selectedFiles.count.value > 0"
                    :files="[...selectedFiles.files.value]"
                    :stats="selectedFiles.stats.value"
                    class="mt-4"
                    @remove="handleFileRemove"
                    @clear="handleFileClear"
                    @thumbnail-loaded="handleThumbnailLoaded"
                    @thumbnail-error="handleThumbnailError"
                />

                <!-- 传输模式选择器 -->
                <ModeSwitcher
                    v-model="transferMode"
                    :online-peer-count="discoveryStore.onlineCount"
                    class="mt-4 mb-4"
                />

                <!-- 发送设置卡片 -->
                <SendSettingsCard
                    v-if="transferMode === 'local'"
                    class="mt-4"
                />

                <!-- 设备列表（本地网络模式） -->
                <PeerList
                    v-if="transferMode === 'local'"
                    :peers="discoveryStore.peerList"
                    :selected-peer-id="selectedPeerId"
                    :loading="discoveryStore.scanning"
                    class="mt-4"
                    @select="handlePeerSelect"
                    @refresh="handlePeerRefresh"
                    @add-manual="handleAddManual"
                />
            </v-col>

            <!-- 右侧：发送任务列表 -->
            <v-col cols="12" md="6">
                <v-card class="mb-4">
                    <v-card-title
                        class="d-flex align-center justify-space-between"
                    >
                        <div class="d-flex align-center">
                            <span>{{ t('send.tasks') }}</span>
                            <v-chip
                                v-if="transferStore.pendingSendTasks.length > 0"
                                color="error"
                                size="small"
                                class="ml-2"
                            >
                                {{ transferStore.pendingSendTasks.length }}
                            </v-chip>
                        </div>
                        <div class="header-actions">
                            <v-btn
                                v-if="transferStore.unifiedSendTasks.length > 0"
                                variant="text"
                                size="x-small"
                                color="error"
                                @click="showClearAllDialog = true"
                            >
                                {{ t('send.task.clearAll') }}
                            </v-btn>
                        </div>
                    </v-card-title>
                    <v-card-text>
                        <!-- 空状态 -->
                        <div
                            v-if="transferStore.unifiedSendTasks.length === 0"
                            class="d-flex flex-column align-center justify-center py-8"
                        >
                            <v-icon
                                :icon="mdiInboxArrowUp"
                                size="64"
                                color="grey"
                                class="mb-4"
                            />
                            <div class="text-h6 text-grey">
                                {{ t('send.noTasks') }}
                            </div>
                            <div class="text-body-2 text-grey">
                                {{ t('send.selectFileToStart') }}
                            </div>
                        </div>

                        <!-- 统一发送任务列表 -->
                        <v-list
                            v-if="transferStore.unifiedSendTasks.length > 0"
                        >
                            <template
                                v-for="(task, index) in visibleSendTasks"
                                :key="task.id"
                            >
                                <v-list-item>
                                    <v-list-item-title>
                                        {{ task.receiverIp }}
                                        <span
                                            v-if="task.receiverLabel"
                                            class="text-grey ml-2"
                                        >
                                            {{ task.receiverLabel }}
                                        </span>
                                    </v-list-item-title>
                                    <v-list-item-subtitle>
                                        {{ formatTime(task.createdAt) }}
                                        <template v-if="task.fileCount > 0">
                                            ·
                                            {{
                                                t('send.task.fileCount', {
                                                    count: task.fileCount,
                                                })
                                            }}
                                            ·
                                            {{ formatFileSize(task.totalSize) }}
                                        </template>
                                        <template
                                            v-if="
                                                task.transferStatus ===
                                                    'transferring' &&
                                                task.progress > 0
                                            "
                                        >
                                            · {{ task.progress }}% ·
                                            {{ formatSpeed(task.speed) }}
                                        </template>
                                    </v-list-item-subtitle>
                                    <template v-slot:append>
                                        <!-- Web 下载任务：显示同意/拒绝按钮 -->
                                        <template
                                            v-if="
                                                task.source === 'webDownload' &&
                                                task.approvalStatus ===
                                                    'pending'
                                            "
                                        >
                                            <v-btn
                                                :icon="mdiCheck"
                                                size="small"
                                                variant="text"
                                                color="success"
                                                @click="
                                                    handleAcceptTask(task.id)
                                                "
                                            />
                                            <v-btn
                                                :icon="mdiClose"
                                                size="small"
                                                variant="text"
                                                color="error"
                                                @click="
                                                    handleRejectTask(task.id)
                                                "
                                            />
                                        </template>
                                        <!-- P2P 任务 pending 状态 -->
                                        <template
                                            v-else-if="
                                                task.source === 'p2p' &&
                                                task.approvalStatus ===
                                                    'pending'
                                            "
                                        >
                                            <v-chip
                                                color="info"
                                                size="small"
                                                label
                                            >
                                                {{
                                                    t(
                                                        'send.task.approval.waitingResponse'
                                                    )
                                                }}
                                            </v-chip>
                                        </template>
                                        <!-- 已接受状态 -->
                                        <template
                                            v-else-if="
                                                task.approvalStatus ===
                                                'accepted'
                                            "
                                        >
                                            <v-chip
                                                color="success"
                                                size="small"
                                                label
                                            >
                                                {{
                                                    t(
                                                        'send.task.approval.accepted'
                                                    )
                                                }}
                                            </v-chip>
                                        </template>
                                        <!-- 已拒绝状态 -->
                                        <template
                                            v-else-if="
                                                task.approvalStatus ===
                                                'rejected'
                                            "
                                        >
                                            <v-chip
                                                color="error"
                                                size="small"
                                                label
                                            >
                                                {{
                                                    t(
                                                        'send.task.approval.rejected'
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
                                                showRemoveTaskDialog(task.id)
                                            "
                                        />
                                    </template>
                                </v-list-item>

                                <!-- 文件发送详情列表 -->
                                <div
                                    v-if="
                                        task.approvalStatus === 'accepted' &&
                                        task.files.length > 0
                                    "
                                    class="upload-records-container ml-4 mr-4 mb-2"
                                >
                                    <!-- 折叠状态：显示前 3 条 -->
                                    <template v-if="!isTaskExpanded(task.id)">
                                        <div
                                            v-for="(
                                                file, fileIndex
                                            ) in task.files.slice(0, 3)"
                                            :key="file.name"
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
                                                        v-if="file.startedAt"
                                                        class="text-caption text-grey ml-2"
                                                        style="
                                                            white-space: nowrap;
                                                        "
                                                    >
                                                        {{
                                                            formatTime(
                                                                file.startedAt
                                                            )
                                                        }}
                                                    </span>
                                                    <span
                                                        class="text-caption text-grey ml-2"
                                                        style="
                                                            white-space: nowrap;
                                                        "
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
                                                            'transferring'
                                                        "
                                                        class="text-body-2 text-grey"
                                                    >
                                                        {{
                                                            formatSpeed(
                                                                file.speed
                                                            )
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
                                                        : file.status ===
                                                            'failed'
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
                                        class="expanded-records"
                                    >
                                        <div
                                            v-for="(
                                                file, fileIndex
                                            ) in task.files.slice(0, 10)"
                                            :key="file.name"
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
                                                        v-if="file.startedAt"
                                                        class="text-caption text-grey ml-2"
                                                        style="
                                                            white-space: nowrap;
                                                        "
                                                    >
                                                        {{
                                                            formatTime(
                                                                file.startedAt
                                                            )
                                                        }}
                                                    </span>
                                                    <span
                                                        class="text-caption text-grey ml-2"
                                                        style="
                                                            white-space: nowrap;
                                                        "
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
                                                            'transferring'
                                                        "
                                                        class="text-body-2 text-grey"
                                                    >
                                                        {{
                                                            formatSpeed(
                                                                file.speed
                                                            )
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
                                                        : file.status ===
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
                                                    t('send.task.moreFiles', {
                                                        count:
                                                            task.files.length -
                                                            3,
                                                    })
                                                }}
                                            </template>
                                            <template v-else>
                                                {{
                                                    t('send.task.collapseFiles')
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
                                    v-if="index < visibleSendTasks.length - 1"
                                />
                            </template>
                        </v-list>

                        <!-- 展开/收起任务列表按钮 -->
                        <div
                            v-if="transferStore.unifiedSendTasks.length > 3"
                            class="text-center mt-2"
                        >
                            <v-btn
                                variant="text"
                                size="small"
                                @click="taskListExpanded = !taskListExpanded"
                            >
                                {{
                                    taskListExpanded
                                        ? t('send.task.collapse')
                                        : t('send.task.expand')
                                }}
                                ({{ transferStore.unifiedSendTasks.length }})
                            </v-btn>
                        </div>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>

        <!-- 发送按钮（P2P 模式） -->
        <v-fab
            v-if="
                selectedFiles.count.value > 0 &&
                selectedPeerId &&
                transferMode === 'local'
            "
            color="primary"
            :icon="mdiSend"
            location="bottom right"
            size="large"
            :loading="sending"
            @click="handleSend"
        />

        <!-- 添加结果提示 -->
        <v-snackbar
            v-model="showAddResult"
            :color="addResultColor"
            :timeout="3000"
        >
            {{ addResultMessage }}
        </v-snackbar>

        <!-- 错误提示 -->
        <v-snackbar v-model="showError" color="error" :timeout="5000">
            {{ errorMessage }}
        </v-snackbar>

        <!-- 移除单个任务确认对话框 -->
        <v-dialog v-model="removeDialog" max-width="400">
            <v-card>
                <v-card-title>{{
                    t('send.task.removeConfirm.title')
                }}</v-card-title>
                <v-card-text>
                    {{ t('send.task.removeConfirm.message') }}
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn variant="text" @click="removeDialog = false">
                        {{ t('common.cancel') }}
                    </v-btn>
                    <v-btn
                        color="error"
                        variant="flat"
                        @click="confirmRemoveTask"
                    >
                        {{ t('common.remove') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

        <!-- 移除全部任务确认对话框 -->
        <v-dialog v-model="showClearAllDialog" max-width="400">
            <v-card>
                <v-card-title>{{
                    t('send.task.clearAllConfirm.title')
                }}</v-card-title>
                <v-card-text>
                    {{
                        t('send.task.clearAllConfirm.message', {
                            count: transferStore.unifiedSendTasks.length,
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
                        @click="confirmClearAllTasks"
                    >
                        {{ t('send.task.clearAll') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>
    </v-container>
</template>

<script setup lang="ts">
import { ref, computed, reactive, onMounted } from 'vue'
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'
import {
    FileSelector,
    ContentTypeSelector,
    ClipboardImporter,
    TextInput,
    MediaPicker,
    FolderPicker,
    AppPicker,
    ModeSwitcher,
    PeerList,
    SelectedFileList,
    SendSettingsCard,
} from '../components/transfer'
import {
    useTransferStore,
    useDiscoveryStore,
    useShareStore,
    useSettingsStore,
} from '../stores'
import { useSelectedFiles } from '../composables'
import type {
    ContentType,
    ContentItem,
    ThumbnailInfo,
    FileSourceType,
} from '../types'
import { FILE_COUNT_LIMIT } from '../types/content'
import { formatFileSize, formatSpeed, formatTime, getFileStatusColor } from '../utils/format'
import {
    mdiInboxArrowUp,
    mdiSend,
    mdiCheck,
    mdiClose,
    mdiDelete,
} from '@mdi/js'

const { t } = useI18n()
const transferStore = useTransferStore()
const discoveryStore = useDiscoveryStore()
const shareStore = useShareStore()
const settingsStore = useSettingsStore()

// 从 store 获取响应式状态（Tab 切换时保留）
const { transferMode, selectedPeerId } = storeToRefs(transferStore)
const { contentType } = storeToRefs(shareStore)

// 使用已选文件管理 composable（现在直接使用 shareStore.selectedFiles）
const selectedFiles = useSelectedFiles()

// 页面激活时验证恢复的状态
onMounted(async () => {
    // 验证选中的设备是否仍在线
    if (selectedPeerId.value) {
        const isOnline = await discoveryStore.checkOnline(selectedPeerId.value)
        if (!isOnline) {
            selectedPeerId.value = ''
        }
    }

    // 自动恢复 Web 下载服务器（应用重启后）
    if (
        settingsStore.webServerSettings.webDownloadEnabled &&
        !shareStore.shareInfo
    ) {
        try {
            const result = await shareStore.startShare([])
            if (result) {
                transferStore.webDownloadEnabled = true
            } else {
                await settingsStore.setWebDownloadState(false)
            }
        } catch {
            await settingsStore.setWebDownloadState(false)
        }
    }
})

// 页面本地状态（无需持久化）
const sending = ref(false)
const showError = ref(false)
const errorMessage = ref('')
const removeDialog = ref(false)
const showClearAllDialog = ref(false)
const taskIdToRemove = ref<string>('')

const showAddResult = ref(false)
const addResultMessage = ref('')
const addResultColor = computed(() => {
    if (addResultMessage.value.includes(t('selectedFiles.duplicate'))) {
        return 'warning'
    }
    if (addResultMessage.value.includes(t('selectedFiles.limitReached'))) {
        return 'error'
    }
    return 'success'
})

function handleContentTypeChange(type: ContentType) {
    contentType.value = type
}

function handleFileSelect(file: {
    path: string
    name: string
    size: number
    type: string
}) {
    const result = selectedFiles.addFile({
        path: file.path,
        name: file.name,
        size: file.size,
        mimeType: file.type,
        sourceType: 'file',
    })
    showAddResultMessage(result)
}

async function handleFolderSelect(
    item: ContentItem & {
        files?: Array<{
            path: string
            name: string
            size: number
            relative_path: string
        }>
    }
) {
    if (item.files && item.files.length > 0) {
        const result = selectedFiles.addFiles(
            item.files.map((f) => ({
                path: f.path,
                name: f.name,
                size: f.size,
                sourceType: 'folder' as FileSourceType,
                relativePath: f.relative_path,
            }))
        )
        showAddResultMessage(result)
    } else {
        const result = selectedFiles.addFile({
            path: item.path,
            name: item.name,
            size: item.size,
            mimeType: item.mimeType,
            sourceType: 'folder',
        })
        showAddResultMessage(result)
    }
}

function handleClipboardSelect(
    item: ContentItem & { content?: string; tempPath?: string }
) {
    const path = item.tempPath || item.path
    const result = selectedFiles.addFile({
        path,
        name: item.name || t('clipboardImporter.content'),
        size: item.size,
        mimeType: 'text/plain',
        sourceType: 'clipboard',
        isTemp: true,
        metadata: item.content ? { content: item.content } : undefined,
    })
    showAddResultMessage(result)
}

function handleTextSelect(
    item: ContentItem & { content?: string; tempPath?: string }
) {
    const path = item.tempPath || item.path
    const result = selectedFiles.addFile({
        path,
        name: item.name || t('textInput.content'),
        size: item.size,
        mimeType: 'text/plain',
        sourceType: 'text',
        isTemp: true,
        metadata: item.content ? { content: item.content } : undefined,
    })
    showAddResultMessage(result)
}

function handleMediaSelect(item: ContentItem) {
    const result = selectedFiles.addFile({
        path: item.path,
        name: item.name,
        size: item.size,
        mimeType: item.mimeType,
        sourceType: 'media',
    })
    showAddResultMessage(result)
}

function handleAppSelect(item: ContentItem) {
    const result = selectedFiles.addFile({
        path: item.path,
        name: item.name,
        size: item.size,
        mimeType: item.mimeType,
        sourceType: 'app',
        metadata: item.metadata,
    })
    showAddResultMessage(result)
}

function showAddResultMessage(
    result:
        | 'added'
        | 'duplicate'
        | 'limit_exceeded'
        | { added: number; duplicates: number; limitExceeded: number }
) {
    if (typeof result === 'string') {
        if (result === 'added') {
            addResultMessage.value = t('selectedFiles.added', { count: 1 })
        } else if (result === 'duplicate') {
            addResultMessage.value = t('selectedFiles.duplicate')
        } else if (result === 'limit_exceeded') {
            addResultMessage.value = t('selectedFiles.limitReached', {
                limit: FILE_COUNT_LIMIT,
            })
        }
    } else {
        const parts: string[] = []
        if (result.added > 0) {
            parts.push(t('selectedFiles.added', { count: result.added }))
        }
        if (result.duplicates > 0) {
            parts.push(
                t('selectedFiles.duplicates', { count: result.duplicates })
            )
        }
        if (result.limitExceeded > 0) {
            parts.push(
                t('selectedFiles.limitExceeded', {
                    count: result.limitExceeded,
                })
            )
        }
        addResultMessage.value =
            parts.join(', ') || t('selectedFiles.noFilesAdded')
    }
    showAddResult.value = true
}

function handleFileRemove(path: string) {
    selectedFiles.removeFile(path)
}

function handleFileClear() {
    selectedFiles.clearFiles()
}

function handleThumbnailLoaded(path: string, thumbnail: ThumbnailInfo) {
    selectedFiles.updateThumbnail(path, thumbnail)
}

function handleThumbnailError(_path: string, _error: string): void {
    void _path
    void _error
}

function handlePeerSelect(peerId: string) {
    selectedPeerId.value = peerId
}

async function handlePeerRefresh() {
    await discoveryStore.refresh()
}

async function handleAddManual(ip: string, port: number) {
    const peer = await discoveryStore.addManual(ip, port)
    if (peer) {
        selectedPeerId.value = peer.id
    }
}

async function handleSend() {
    if (selectedFiles.count.value === 0 || !selectedPeerId.value) return

    const peer = discoveryStore.selectedPeer
    if (!peer) {
        showError.value = true
        errorMessage.value = t('send.selectTargetDevice')
        return
    }

    sending.value = true

    try {
        const filesToSend = [...selectedFiles.files.value]
        for (const file of filesToSend) {
            const metadata = await transferStore.prepareTransfer(file.path)
            if (!metadata) {
                throw new Error(t('send.prepareFailed'))
            }
            await transferStore.send(metadata, peer.id, peer.ip, peer.port)
        }
        selectedFiles.clearFiles()
        selectedPeerId.value = ''
    } catch (error) {
        showError.value = true
        errorMessage.value = t('send.sendFailed', { error })
    } finally {
        sending.value = false
    }
}

// Task list management
const taskListExpanded = ref(false)
const expandedTasks = reactive(new Set<string>())

const visibleSendTasks = computed(() => {
    const list = transferStore.unifiedSendTasks
    if (taskListExpanded.value || list.length <= 3) {
        return list
    }
    return list.slice(0, 3)
})

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

function showRemoveTaskDialog(taskId: string) {
    taskIdToRemove.value = taskId
    removeDialog.value = true
}

async function confirmRemoveTask() {
    const taskId = taskIdToRemove.value
    // 移除 Web 下载任务
    if (taskId.startsWith('web-')) {
        transferStore.deleteSendTaskItem(taskId)
    }
    // 移除 P2P 任务
    if (taskId.startsWith('p2p-')) {
        const originalId = taskId.replace('p2p-', '')
        transferStore.tasks.delete(originalId)
    }
    removeDialog.value = false
    taskIdToRemove.value = ''
}

async function confirmClearAllTasks() {
    // 清理所有 Web 下载任务
    transferStore.clearSendTaskItems()
    // 清理所有发送方向的 P2P 任务
    for (const [id, task] of transferStore.tasks.entries()) {
        if (task.direction === 'send') {
            transferStore.tasks.delete(id)
        }
    }
    showClearAllDialog.value = false
}

async function handleAcceptTask(taskId: string) {
    try {
        // taskId 格式为 "web-{requestId}"，需要提取原始请求 ID
        const requestId = taskId.startsWith('web-') ? taskId.slice(4) : taskId
        await shareStore.acceptRequest(requestId)
    } catch (error) {
        showError.value = true
        errorMessage.value = t('send.task.acceptError', { error })
    }
}

async function handleRejectTask(taskId: string) {
    try {
        // taskId 格式为 "web-{requestId}"，需要提取原始请求 ID
        const requestId = taskId.startsWith('web-') ? taskId.slice(4) : taskId
        await shareStore.rejectRequest(requestId)
    } catch (error) {
        showError.value = true
        errorMessage.value = t('send.task.rejectError', { error })
    }
}


function getFileStatusText(status: string): string {
    const keyMap: Record<string, string> = {
        pending: 'send.task.fileStatus.pending',
        transferring: 'send.task.fileStatus.transferring',
        completed: 'send.task.fileStatus.completed',
        failed: 'send.task.fileStatus.failed',
        cancelled: 'send.task.fileStatus.cancelled',
        interrupted: 'send.task.fileStatus.interrupted',
    }
    return t(keyMap[status] || 'send.task.fileStatus.pending')
}
</script>

<style scoped>
.send-view {
    max-width: 1400px;
    margin: 0 auto;
}

.header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
}

.header-actions :deep(.v-btn__content) {
    font-size: 14px;
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
