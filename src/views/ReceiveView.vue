<!-- 文件接收页面 -->
<template>
    <v-container fluid class="receive-view">
        <v-row>
            <!-- 左侧：接收设置 -->
            <v-col cols="12" md="6" class="settings-col">
                <!-- 接收模式选择器 -->
                <ReceiveModeSelector />

                <!-- 接收设置卡片 -->
                <ReceiveSettingsCard />

                <!-- 控制按钮 -->
                <v-card>
                    <v-card-text>
                        <v-btn
                            v-if="!isReceiving"
                            color="success"
                            variant="flat"
                            block
                            size="large"
                            :loading="starting"
                            @click="handleStartReceiving"
                        >
                            <v-icon :icon="mdiWifiPlus" class="mr-2" />
                            {{ t('receive.startReceiving') }}
                        </v-btn>
                        <v-btn
                            v-else
                            color="error"
                            variant="flat"
                            block
                            size="large"
                            :loading="stopping"
                            @click="handleStopReceiving"
                        >
                            <v-icon :icon="mdiWifiOff" class="mr-2" />
                            {{ t('receive.stopReceiving') }}
                        </v-btn>
                    </v-card-text>
                </v-card>
            </v-col>

            <!-- 右侧：接收任务列表 -->
            <v-col cols="12" md="6">
                <v-card class="mb-4">
                    <v-card-title
                        class="d-flex align-center justify-space-between"
                    >
                        <div class="d-flex align-center">
                            <span>{{ t('receive.tasks') }}</span>
                            <v-chip
                                v-if="
                                    transferStore.pendingReceiveTasks.length > 0
                                "
                                color="error"
                                size="small"
                                class="ml-2"
                            >
                                {{ transferStore.pendingReceiveTasks.length }}
                            </v-chip>
                        </div>
                        <div class="header-actions">
                            <v-btn
                                v-if="transferStore.unifiedReceiveTasks.length > 0"
                                variant="text"
                                size="x-small"
                                color="error"
                                @click="showClearAllDialog = true"
                            >
                                {{ t('receive.task.clearAll') }}
                            </v-btn>
                        </div>
                    </v-card-title>
                    <v-card-text>
                        <!-- 空状态 -->
                        <div
                            v-if="
                                transferStore.unifiedReceiveTasks.length === 0
                            "
                            class="d-flex flex-column align-center justify-center py-8"
                        >
                            <v-icon
                                :icon="mdiInboxArrowDown"
                                size="64"
                                color="grey"
                                class="mb-4"
                            />
                            <div class="text-h6 text-grey">
                                {{ t('receive.noTasks') }}
                            </div>
                            <div class="text-body-2 text-grey">
                                {{
                                    isReceiving
                                        ? t('receive.waitingForSender')
                                        : t('receive.clickToStart')
                                }}
                            </div>
                        </div>

                        <!-- 统一接收任务列表 -->
                        <v-list
                            v-if="transferStore.unifiedReceiveTasks.length > 0"
                        >
                            <template
                                v-for="(task, index) in visibleReceiveTasks"
                                :key="task.id"
                            >
                                <v-list-item>
                                    <v-list-item-title>
                                        {{ task.senderIp }}
                                        <span
                                            v-if="task.senderLabel"
                                            class="text-grey ml-2"
                                        >
                                            {{ task.senderLabel }}
                                        </span>
                                    </v-list-item-title>
                                    <v-list-item-subtitle>
                                        {{ formatTime(task.createdAt) }}
                                        <template v-if="task.files.length > 0">
                                            ·
                                            {{
                                                t('receive.task.fileCount', {
                                                    count: task.files.length,
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
                                        <!-- 待处理状态：显示同意/拒绝按钮 -->
                                        <template
                                            v-if="
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
                                                        'receive.task.approval.accepted'
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
                                                        'receive.task.approval.rejected'
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

                                <!-- 文件下载详情列表 -->
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
                                                    t(
                                                        'receive.task.moreFiles',
                                                        {
                                                            count:
                                                                task.files
                                                                    .length - 3,
                                                        }
                                                    )
                                                }}
                                            </template>
                                            <template v-else>
                                                {{
                                                    t(
                                                        'receive.task.collapseFiles'
                                                    )
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
                                    v-if="
                                        index < visibleReceiveTasks.length - 1
                                    "
                                />
                            </template>
                        </v-list>

                        <!-- 展开/收起任务列表按钮 -->
                        <div
                            v-if="transferStore.unifiedReceiveTasks.length > 3"
                            class="text-center mt-2"
                        >
                            <v-btn
                                variant="text"
                                size="small"
                                @click="taskListExpanded = !taskListExpanded"
                            >
                                {{
                                    taskListExpanded
                                        ? t('receive.task.collapse')
                                        : t('receive.task.expand')
                                }}
                                ({{ transferStore.unifiedReceiveTasks.length }})
                            </v-btn>
                        </div>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>

        <!-- 移除单个任务确认对话框 -->
        <v-dialog v-model="removeDialog" max-width="400">
            <v-card>
                <v-card-title>{{
                    t('receive.task.removeConfirm.title')
                }}</v-card-title>
                <v-card-text>
                    {{ t('receive.task.removeConfirm.message') }}
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
                    t('receive.task.clearAllConfirm.title')
                }}</v-card-title>
                <v-card-text>
                    {{
                        t('receive.task.clearAllConfirm.message', {
                            count: transferStore.unifiedReceiveTasks.length,
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
                        {{ t('receive.task.clearAll') }}
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
import { ref, computed, reactive, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
    ReceiveModeSelector,
    ReceiveSettingsCard,
} from '../components/transfer'
import { useTransferStore, useSettingsStore } from '../stores'
import { formatFileSize, formatSpeed, formatTime, getFileStatusColor } from '../utils/format'
import {
    mdiWifiOff,
    mdiWifiPlus,
    mdiInboxArrowDown,
    mdiCheck,
    mdiClose,
    mdiDelete,
} from '@mdi/js'

const { t } = useI18n()
const transferStore = useTransferStore()
const settingsStore = useSettingsStore()

// 页面本地状态（无需持久化）
const starting = ref(false)
const stopping = ref(false)
const showError = ref(false)
const errorMessage = ref('')
const removeDialog = ref(false)
const showClearAllDialog = ref(false)
const taskIdToRemove = ref<string>('')

const isReceiving = computed(() => transferStore.receivePort > 0)

/** 任务列表是否展开 */
const taskListExpanded = ref(false)

/** 已展开文件列表的任务 ID 集合 */
const expandedTasks = reactive(new Set<string>())

/** 可见的接收任务列表（折叠时最多显示 3 个） */
const visibleReceiveTasks = computed(() => {
    const list = transferStore.unifiedReceiveTasks
    if (taskListExpanded.value || list.length <= 3) {
        return list
    }
    return list.slice(0, 3)
})

/** 判断某个任务的文件列表是否展开 */
function isTaskExpanded(taskId: string): boolean {
    return expandedTasks.has(taskId)
}


/** 获取文件状态文本 */
function getFileStatusText(status: string): string {
    const keyMap: Record<string, string> = {
        pending: 'receive.task.fileStatus.pending',
        transferring: 'receive.task.fileStatus.transferring',
        completed: 'receive.task.fileStatus.completed',
        failed: 'receive.task.fileStatus.failed',
        cancelled: 'receive.task.fileStatus.cancelled',
        interrupted: 'receive.task.fileStatus.interrupted',
    }
    return t(keyMap[status] || 'receive.task.fileStatus.pending')
}

async function handleStartReceiving() {
    starting.value = true
    showError.value = false

    try {
        await transferStore.startReceiving()
    } catch (error) {
        showError.value = true
        errorMessage.value = t('receive.startFailed', { error })
    } finally {
        starting.value = false
    }
}

async function handleStopReceiving() {
    stopping.value = true
    showError.value = false

    try {
        await transferStore.stopReceiving()
    } catch (error) {
        showError.value = true
        errorMessage.value = t('receive.stopFailed', { error })
    } finally {
        stopping.value = false
    }
}

async function handleAcceptTask(taskId: string) {
    try {
        await transferStore.acceptReceiveTask(taskId)
    } catch (error) {
        showError.value = true
        errorMessage.value = t('receive.task.acceptError', { error })
    }
}

async function handleRejectTask(taskId: string) {
    try {
        await transferStore.rejectReceiveTask(taskId)
    } catch (error) {
        showError.value = true
        errorMessage.value = t('receive.task.rejectError', { error })
    }
}

/** 显示移除单个任务对话框 */
function showRemoveTaskDialog(taskId: string) {
    taskIdToRemove.value = taskId
    removeDialog.value = true
}

/** 确认移除单个任务 */
function confirmRemoveTask() {
    const taskId = taskIdToRemove.value
    // 移除 Web 上传任务
    if (taskId.startsWith('web-')) {
        transferStore.deleteReceiveTaskItem(taskId)
    }
    // 移除 P2P 任务
    if (taskId.startsWith('p2p-')) {
        const originalId = taskId.replace('p2p-', '')
        transferStore.tasks.delete(originalId)
    }
    removeDialog.value = false
    taskIdToRemove.value = ''
}

/** 确认移除全部任务 */
function confirmClearAllTasks() {
    // 清理所有 Web 上传任务
    transferStore.clearReceiveTaskItems()
    // 清理所有接收方向的 P2P 任务
    for (const [id, task] of transferStore.tasks.entries()) {
        if (task.direction === 'receive') {
            transferStore.tasks.delete(id)
        }
    }
    showClearAllDialog.value = false
}

function toggleFileList(taskId: string) {
    if (expandedTasks.has(taskId)) {
        expandedTasks.delete(taskId)
    } else {
        expandedTasks.add(taskId)
    }
}


onMounted(async () => {
    await transferStore.initialize()
    // 进入页面自动启动 P2P 接收服务器
    await autoStartReceiving()

    // 自动恢复 Web 上传服务器（应用重启后）
    if (
        settingsStore.webServerSettings.webUploadEnabled &&
        !transferStore.webUploadEnabled
    ) {
        try {
            await transferStore.startWebUpload()
        } catch {
            await settingsStore.setWebUploadState(false)
        }
    }
})

onUnmounted(async () => {
    // 离开页面时检查活跃的 P2P 任务后关闭 P2P 接收服务器
    await autoStopReceiving()
    // 仅重置页面级状态，不清除 Web 服务状态和任务记录
    transferStore.resetPageState()
})

/**
 * 自动启动接收服务器
 */
async function autoStartReceiving() {
    // 如果已经在接收，不重复启动
    if (transferStore.receivePort > 0) {
        return
    }

    starting.value = true
    showError.value = false

    try {
        await transferStore.startReceiving()
    } catch (error) {
        showError.value = true
        errorMessage.value = t('receive.startFailed', { error })
    } finally {
        starting.value = false
    }
}

/**
 * 自动停止接收服务器（有活跃任务时保持运行）
 */
async function autoStopReceiving() {
    // 如果没有在接收，直接返回
    if (transferStore.receivePort === 0) {
        return
    }

    // 检查是否有活跃任务（正在传输或等待中）
    const hasActiveTasks = transferStore.receiveTasks.some(
        (task) => task.status === 'transferring' || task.status === 'pending'
    )

    // 有活跃任务时保持服务器运行
    if (hasActiveTasks) {
        return
    }

    // 无活跃任务，关闭服务器
    try {
        await transferStore.stopReceiving()
    } catch (error) {
        // 静默处理错误，不影响页面离开
        console.error('停止接收失败:', error)
    }
}
</script>

<style scoped>
.receive-view {
    min-height: calc(100vh - 64px);
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

/* 左侧设置列的卡片间距 */
.settings-col > * {
    margin-bottom: 16px;
}

.settings-col > *:last-child {
    margin-bottom: 0;
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

/* 文件下载记录容器（与 ShareLinkView 一致） */
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
