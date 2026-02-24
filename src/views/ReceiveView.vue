<!-- 文件接收页面 -->
<template>
    <v-container fluid class="receive-view">
        <v-row>
            <!-- 左侧：接收设置 -->
            <v-col cols="12" md="6">
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

            <!-- 右侧：接收任务进度 -->
            <v-col cols="12" md="6">
                <v-card class="mb-4">
                    <v-card-title
                        class="d-flex align-center justify-space-between"
                    >
                        <span>{{ t('receive.tasks') }}</span>
                        <v-btn
                            v-if="transferStore.receiveTasks.length > 0"
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
                    v-if="transferStore.receiveTasks.length === 0"
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

                <!-- 任务列表 -->
                <ProgressDisplay
                    v-for="task in transferStore.receiveTasks"
                    :key="task.id"
                    :task="task"
                    class="mb-4"
                    @cancel="handleCancel"
                    @retry="handleRetry"
                    @remove="handleRemove"
                />
            </v-col>
        </v-row>

        <!-- 错误提示 -->
        <v-snackbar v-model="showError" color="error" :timeout="5000">
            {{ errorMessage }}
        </v-snackbar>
    </v-container>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
    ProgressDisplay,
    ReceiveModeSelector,
    ReceiveSettingsCard,
} from '../components/transfer'
import { useTransferStore } from '../stores'
import { mdiWifiOff, mdiWifiPlus, mdiInboxArrowDown } from '@mdi/js'

const { t } = useI18n()
const transferStore = useTransferStore()

// 页面本地状态（无需持久化）
const starting = ref(false)
const stopping = ref(false)
const showError = ref(false)
const errorMessage = ref('')

const isReceiving = computed(() => transferStore.receivePort > 0)

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

async function handleCancel(taskId: string) {
    await transferStore.cancel(taskId)
}

async function handleRetry() {
    // 接收任务暂不支持重试
    showError.value = true
    errorMessage.value = t('receive.retryNotSupported')
}

function handleRemove(taskId: string) {
    transferStore.tasks.delete(taskId)
}

async function handleCleanup() {
    await transferStore.cleanup()
}

onMounted(async () => {
    await transferStore.initialize()
    // 进入页面自动启动接收服务器
    await autoStartReceiving()
})

onUnmounted(async () => {
    // 离开页面时检查活跃任务后关闭服务器
    await autoStopReceiving()
    transferStore.destroy()
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
