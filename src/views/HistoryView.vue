<!-- 历史记录页面 -->
<template>
    <v-container fluid class="history-view">
        <v-card>
            <v-card-title class="d-flex align-center justify-space-between">
                <span>传输历史</span>
                <v-btn
                    v-if="history.length > 0"
                    color="error"
                    variant="text"
                    size="small"
                    @click="handleClearAll"
                >
                    清空历史
                </v-btn>
            </v-card-title>

            <v-divider />

            <!-- 空状态 -->
            <v-card-text
                v-if="history.length === 0"
                class="d-flex flex-column align-center justify-center py-16"
            >
                <v-icon
                    :icon="mdiHistory"
                    size="64"
                    color="grey"
                    class="mb-4"
                />
                <div class="text-h6 text-grey">暂无传输历史</div>
                <div class="text-body-2 text-grey">
                    完成文件传输后将显示在此处
                </div>
            </v-card-text>

            <!-- 历史列表 -->
            <v-list v-else density="compact">
                <template v-for="(item, index) in history" :key="item.id">
                    <v-list-item>
                        <template #prepend>
                            <v-avatar
                                :color="getStatusColor(item.status)"
                                size="40"
                            >
                                <v-icon
                                    :icon="getDirectionIcon(item.direction)"
                                    color="white"
                                    size="20"
                                />
                            </v-avatar>
                        </template>

                        <v-list-item-title>{{
                            item.fileName
                        }}</v-list-item-title>
                        <v-list-item-subtitle>
                            <span class="mr-2">{{
                                formatSize(item.fileSize)
                            }}</span>
                            <span class="mr-2">•</span>
                            <span class="mr-2">{{ item.peerName }}</span>
                            <span class="mr-2">•</span>
                            <span>{{ formatTime(item.completedAt) }}</span>
                        </v-list-item-subtitle>

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

                    <v-divider v-if="index < history.length - 1" />
                </template>
            </v-list>
        </v-card>
    </v-container>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import type { TaskStatus, TransferDirection } from '../types'
import { getStatusText as getStatusTextFn, formatFileSize } from '../types'
import { mdiHistory, mdiArrowUp, mdiArrowDown } from '@mdi/js'

interface HistoryItem {
    id: string
    fileName: string
    fileSize: number
    peerName: string
    status: TaskStatus
    direction: TransferDirection
    completedAt: number
}

const history = ref<HistoryItem[]>([])

function getStatusColor(status: TaskStatus): string {
    const colors: Record<TaskStatus, string> = {
        pending: 'grey',
        transferring: 'primary',
        completed: 'success',
        failed: 'error',
        cancelled: 'warning',
    }
    return colors[status]
}

function getStatusText(status: TaskStatus): string {
    return getStatusTextFn(status)
}

function getDirectionIcon(direction: TransferDirection): string {
    return direction === 'send' ? mdiArrowUp : mdiArrowDown
}

function formatSize(bytes: number): string {
    return formatFileSize(bytes)
}

function formatTime(timestamp: number): string {
    if (!timestamp) return '--'
    const date = new Date(timestamp)
    return date.toLocaleString('zh-CN')
}

async function handleClearAll() {
    history.value = []
    // 持久化存储清空
    localStorage.removeItem('transfer-history')
}

function loadHistory() {
    const saved = localStorage.getItem('transfer-history')
    if (saved) {
        try {
            history.value = JSON.parse(saved)
        } catch {
            history.value = []
        }
    }
}

function saveHistory() {
    localStorage.setItem('transfer-history', JSON.stringify(history.value))
}

// 监听传输完成事件，添加到历史
function setupHistoryListener() {
    // 从 transferStore 获取已完成的任务并添加到历史

    const transferStore = (window as any).__TRANSFER_STORE__
    if (transferStore) {
        const completedTasks = transferStore.completedTasks
        for (const task of completedTasks) {
            if (!history.value.find((h) => h.id === task.id)) {
                history.value.unshift({
                    id: task.id,
                    fileName: task.file.name,
                    fileSize: task.file.size,
                    peerName: task.peer?.name || '未知设备',
                    status: task.status,
                    direction: task.direction,
                    completedAt: task.completedAt || Date.now(),
                })
            }
        }
        saveHistory()
    }
}

onMounted(() => {
    loadHistory()
    // 定期检查并更新历史
    setInterval(setupHistoryListener, 5000)
})
</script>

<style scoped>
.history-view {
    max-width: 800px;
    margin: 0 auto;
}
</style>
