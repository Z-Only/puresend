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
                    @clear="handleFileClear"
                />

                <!-- 文件夹选择 -->
                <FolderPicker
                    v-else-if="contentType === 'folder'"
                    @select="handleContentSelect"
                />

                <!-- 剪贴板导入 -->
                <ClipboardImporter
                    v-else-if="contentType === 'clipboard'"
                    @select="handleContentSelect"
                />

                <!-- 文本输入 -->
                <TextInput
                    v-else-if="contentType === 'text'"
                    @select="handleContentSelect"
                />

                <!-- 媒体选择 -->
                <MediaPicker
                    v-else-if="contentType === 'media'"
                    @select="handleContentSelect"
                />

                <!-- 应用选择 -->
                <AppPicker
                    v-else-if="contentType === 'app'"
                    @select="handleContentSelect"
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
                        <span>发送任务</span>
                        <v-btn
                            v-if="transferStore.sendTasks.length > 0"
                            color="primary"
                            variant="text"
                            size="small"
                            @click="handleCleanup"
                        >
                            清理已完成
                        </v-btn>
                    </v-card-title>
                </v-card>

                <!-- 空状态 -->
                <div
                    v-if="transferStore.sendTasks.length === 0"
                    class="d-flex flex-column align-center justify-center py-8"
                >
                    <v-icon
                        icon="mdi-inbox-arrow-up"
                        size="64"
                        color="grey"
                        class="mb-4"
                    />
                    <div class="text-h6 text-grey">暂无发送任务</div>
                    <div class="text-body-2 text-grey">选择文件开始发送</div>
                </div>

                <!-- 任务列表 -->
                <ProgressDisplay
                    v-for="task in transferStore.sendTasks"
                    :key="task.id"
                    :task="task"
                    class="mb-4"
                    @cancel="handleCancel"
                    @retry="handleRetry"
                    @remove="handleRemove"
                />
            </v-col>
        </v-row>

        <!-- 发送按钮 -->
        <v-fab
            v-if="(selectedFile || selectedContent) && selectedPeerId"
            color="primary"
            icon="mdi-send"
            location="bottom right"
            size="large"
            :loading="sending"
            @click="handleSend"
        />

        <!-- 错误提示 -->
        <v-snackbar v-model="showError" color="error" :timeout="5000">
            {{ errorMessage }}
        </v-snackbar>
    </v-container>
</template>

<script setup lang="ts">
import { ref } from 'vue'
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
} from '../components/transfer'
import { useTransferStore, useDiscoveryStore } from '../stores'
import type {
    TransferMode,
    TransferTask,
    ContentType,
    ContentItem,
} from '../types'

const transferStore = useTransferStore()
const discoveryStore = useDiscoveryStore()

const contentType = ref<ContentType>('file')
const selectedFile = ref<{
    path: string
    name: string
    size: number
    type: string
} | null>(null)
const selectedContent = ref<ContentItem | null>(null)
const selectedPeerId = ref('')
const transferMode = ref<TransferMode>('local')
const sending = ref(false)
const showError = ref(false)
const errorMessage = ref('')

function handleContentTypeChange(type: ContentType) {
    contentType.value = type
    selectedFile.value = null
    selectedContent.value = null
}

async function handleFileSelect(file: {
    path: string
    name: string
    size: number
    type: string
}) {
    selectedFile.value = file
}

function handleFileClear() {
    selectedFile.value = null
}

async function handleContentSelect(item: ContentItem) {
    selectedContent.value = item
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
    const targetPath = selectedFile.value?.path || selectedContent.value?.path
    if (!targetPath || !selectedPeerId.value) return

    const peer = discoveryStore.selectedPeer
    if (!peer) {
        showError.value = true
        errorMessage.value = '请选择目标设备'
        return
    }

    sending.value = true

    try {
        // 准备文件传输（计算哈希等）
        const metadata = await transferStore.prepareTransfer(targetPath)
        if (!metadata) {
            throw new Error('准备传输失败')
        }

        // 发送文件
        const taskId = await transferStore.send(
            metadata,
            peer.id,
            peer.ip,
            peer.port
        )

        if (taskId) {
            // 清除选择
            selectedFile.value = null
            selectedContent.value = null
            selectedPeerId.value = ''
        }
    } catch (error) {
        showError.value = true
        errorMessage.value = `发送失败：${error}`
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

function handleRemove(taskId: string) {
    // 从列表中移除任务
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
