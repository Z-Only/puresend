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

                <!-- 传输模式选择 -->
                <ModeSwitcher
                    v-model="transferMode"
                    :online-peer-count="discoveryStore.onlineCount"
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

            <!-- 右侧：发送任务进度 -->
            <v-col cols="12" md="6">
                <v-card class="mb-4">
                    <v-card-title
                        class="d-flex align-center justify-space-between"
                    >
                        <span>{{ t('send.tasks') }}</span>
                        <v-btn
                            v-if="transferStore.sendTasks.length > 0"
                            color="primary"
                            variant="text"
                            size="small"
                            @click="handleCleanup"
                        >
                            {{ t('send.cleanup') }}
                        </v-btn>
                    </v-card-title>
                </v-card>

                <!-- 空状态 -->
                <div
                    v-if="transferStore.sendTasks.length === 0"
                    class="d-flex flex-column align-center justify-center py-8"
                >
                    <v-icon
                        :icon="mdiInboxArrowUp"
                        size="64"
                        color="grey"
                        class="mb-4"
                    />
                    <div class="text-h6 text-grey">{{ t('send.noTasks') }}</div>
                    <div class="text-body-2 text-grey">
                        {{ t('send.selectFileToStart') }}
                    </div>
                </div>

                <!-- 任务列表 -->
                <ProgressDisplay
                    v-for="task in transferStore.sendTasks"
                    :key="task.id"
                    :task="task"
                    class="mb-4"
                    @cancel="handleCancel"
                    @retry="handleRetry"
                    @remove="handleRemoveTask"
                />
            </v-col>
        </v-row>

        <!-- 发送按钮 -->
        <v-fab
            v-if="selectedFiles.count.value > 0 && selectedPeerId"
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
    </v-container>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import {
    FileSelector,
    ContentTypeSelector,
    ClipboardImporter,
    TextInput,
    MediaPicker,
    FolderPicker,
    AppPicker,
    ProgressDisplay,
    ModeSwitcher,
    PeerList,
    SelectedFileList,
} from '../components/transfer'
import { useTransferStore, useDiscoveryStore } from '../stores'
import { useSelectedFiles } from '../composables'
import type {
    TransferMode,
    ContentType,
    ContentItem,
    ThumbnailInfo,
    FileSourceType,
    TransferTask,
} from '../types'
import { FILE_COUNT_LIMIT } from '../types/content'
import { mdiInboxArrowUp, mdiSend } from '@mdi/js'

const { t } = useI18n()
const transferStore = useTransferStore()
const discoveryStore = useDiscoveryStore()

// 使用已选文件管理 composable
const selectedFiles = useSelectedFiles()

const contentType = ref<ContentType>('file')
const selectedPeerId = ref('')
const transferMode = ref<TransferMode>('local')
const sending = ref(false)
const showError = ref(false)
const errorMessage = ref('')

// 添加结果提示
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

// 内容类型切换时不再清空已选文件
function handleContentTypeChange(type: ContentType) {
    contentType.value = type
}

// 处理文件选择
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

// 处理文件夹选择（展开为文件列表）
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
    // 如果有文件列表，直接添加
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
        // 单个文件夹项
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

// 处理剪贴板选择（生成临时文本文件）
function handleClipboardSelect(
    item: ContentItem & { content?: string; tempPath?: string }
) {
    // 如果有临时文件路径，使用它
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

// 处理文本输入选择
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

// 处理媒体文件选择
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

// 处理应用选择
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

// 显示添加结果消息
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

// 移除单个文件
function handleFileRemove(path: string) {
    selectedFiles.removeFile(path)
}

// 清空所有文件
function handleFileClear() {
    selectedFiles.clearFiles()
}

// 缩略图加载完成
function handleThumbnailLoaded(path: string, thumbnail: ThumbnailInfo) {
    selectedFiles.updateThumbnail(path, thumbnail)
}

// 缩略图加载失败
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
        // 发送所有选中的文件
        const filesToSend = [...selectedFiles.files.value]
        for (const file of filesToSend) {
            // 准备文件传输（计算哈希等）
            const metadata = await transferStore.prepareTransfer(file.path)
            if (!metadata) {
                throw new Error(t('send.prepareFailed'))
            }

            // 发送文件
            await transferStore.send(metadata, peer.id, peer.ip, peer.port)
        }

        // 清除选择
        selectedFiles.clearFiles()
        selectedPeerId.value = ''
    } catch (error) {
        showError.value = true
        errorMessage.value = t('send.sendFailed', { error })
    } finally {
        sending.value = false
    }
}

async function handleCancel(taskId: string) {
    await transferStore.cancel(taskId)
}

async function handleRetry(task: TransferTask) {
    if (task.peer && task.file.path) {
        const metadata = await transferStore.prepareTransfer(task.file.path)
        if (metadata) {
            await transferStore.send(
                metadata,
                task.peer.id,
                task.peer.ip,
                task.peer.port
            )
        }
    }
}

function handleRemoveTask(taskId: string) {
    transferStore.tasks.delete(taskId)
}

async function handleCleanup() {
    await transferStore.cleanup()
}
</script>

<style scoped>
.send-view {
    min-height: calc(100vh - 64px);
}

/* 修复按钮中文本居中问题 */
.v-btn:deep(.v-btn__content) {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    justify-items: center;
    width: 100%;
}

.v-btn:deep(.v-btn__content .v-icon) {
    grid-column: 1;
}

.v-btn:deep(.v-btn__content span) {
    grid-column: 2;
    text-align: center;
}
</style>
