<!-- 文件接收页面 -->
<template>
    <v-container fluid class="receive-view">
        <v-row>
            <!-- 左侧：网络信息和接收设置 -->
            <v-col cols="12" md="6">
                <!-- 网络信息卡片 -->
                <v-card class="mb-4">
                    <v-card-title
                        class="text-subtitle-1 d-flex align-center justify-space-between"
                    >
                        <span>{{ t('receive.networkInfo') }}</span>
                        <v-btn
                            color="primary"
                            variant="text"
                            size="small"
                            :loading="loadingNetworkInfo"
                            @click="handleShowNetworkInfo"
                        >
                            <v-icon :icon="mdiWifi" class="mr-2" />
                            {{ t('receive.viewNetworkInfo') }}
                        </v-btn>
                    </v-card-title>
                    <v-card-text v-if="showNetworkInfo">
                        <NetworkInfo
                            :network-address="transferStore.networkAddress"
                            :port="transferStore.receivePort"
                            :share-code="transferStore.shareCode"
                        />
                    </v-card-text>
                    <v-card-text v-else class="text-center py-4">
                        <v-icon :icon="mdiWifiOff" size="48" color="grey" />
                        <div class="text-body-2 text-grey mt-2">
                            {{ t('receive.clickToViewNetwork') }}
                        </div>
                    </v-card-text>
                </v-card>

                <!-- 接收模式选择 -->
                <v-card class="mb-4">
                    <v-card-title class="text-subtitle-1">
                        {{ t('receive.mode') }}
                    </v-card-title>
                    <v-card-text>
                        <v-btn-toggle
                            v-model="receiveMode"
                            color="primary"
                            variant="outlined"
                            mandatory
                            class="w-100"
                        >
                            <v-btn value="local" block>
                                <v-icon :icon="mdiWifi" class="mr-2" />
                                {{ t('receive.localNetwork') }}
                            </v-btn>
                            <v-btn value="cloud" block disabled>
                                <v-icon :icon="mdiCloud" class="mr-2" />
                                {{ t('receive.cloudTransferDev') }}
                            </v-btn>
                        </v-btn-toggle>
                    </v-card-text>
                </v-card>

                <!-- 接收目录设置 -->
                <v-card class="mb-4">
                    <v-card-title class="text-subtitle-1">
                        {{ t('receive.receiveDirectory') }}
                    </v-card-title>
                    <v-card-text>
                        <v-text-field
                            :model-value="transferStore.receiveDirectory"
                            :label="t('receive.saveLocation')"
                            readonly
                            variant="outlined"
                            density="compact"
                            :append-icon="mdiFolderOpen"
                            @click:append="handleSelectDirectory"
                        />
                    </v-card-text>
                </v-card>

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
import { open } from '@tauri-apps/plugin-dialog'
import { ProgressDisplay } from '../components/transfer'
import NetworkInfo from '../components/transfer/NetworkInfo.vue'
import { useTransferStore } from '../stores'
import type { TransferMode } from '../types'
import {
    mdiWifi,
    mdiWifiOff,
    mdiCloud,
    mdiWifiPlus,
    mdiInboxArrowDown,
    mdiFolderOpen,
} from '@mdi/js'

const { t } = useI18n()
const transferStore = useTransferStore()

const receiveMode = ref<TransferMode>('local')
const starting = ref(false)
const stopping = ref(false)
const showError = ref(false)
const errorMessage = ref('')
const showNetworkInfo = ref(false)
const loadingNetworkInfo = ref(false)

const isReceiving = computed(() => transferStore.receivePort > 0)

async function handleShowNetworkInfo() {
    if (showNetworkInfo.value && transferStore.networkAddress) {
        // 如果已显示且有网络信息，直接返回
        return
    }

    loadingNetworkInfo.value = true
    showError.value = false

    try {
        // 查看网络信息时调用 getNetworkInfo 获取真实信息，但不启动接收服务
        await transferStore.getNetworkInfo()
        showNetworkInfo.value = true
    } catch (error) {
        showError.value = true
        errorMessage.value = t('receive.getNetworkInfoFailed', { error })
    } finally {
        loadingNetworkInfo.value = false
    }
}

async function handleStartReceiving() {
    starting.value = true
    showError.value = false

    try {
        await transferStore.startReceiving()
        // 启动接收后不自动显示网络信息，用户需要时可手动点击"查看网络信息"
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
        // 停止接收后隐藏网络信息
        showNetworkInfo.value = false
    } catch (error) {
        showError.value = true
        errorMessage.value = t('receive.stopFailed', { error })
    } finally {
        stopping.value = false
    }
}

async function handleSelectDirectory() {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            title: t('receive.selectDirectory'),
        })

        if (selected && typeof selected === 'string') {
            await transferStore.updateReceiveDirectory(selected)
        }
    } catch (error) {
        showError.value = true
        errorMessage.value = t('receive.selectDirectoryFailed', { error })
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
})

onUnmounted(() => {
    transferStore.destroy()
})
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
