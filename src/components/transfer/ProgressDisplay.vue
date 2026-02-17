<!-- 传输进度显示组件 -->
<template>
    <v-card class="progress-display">
        <v-card-text>
            <!-- 任务状态和文件名 -->
            <div class="d-flex align-center mb-4">
                <v-icon
                    :icon="statusIcon"
                    :color="statusColor"
                    size="32"
                    class="mr-3"
                />
                <div class="flex-grow-1 overflow-hidden">
                    <div class="text-subtitle-1 text-truncate">
                        {{ task?.file.name || '未知文件' }}
                    </div>
                    <div class="text-body-2 text-grey">
                        {{ getStatusText(task?.status || 'pending') }}
                        <span v-if="task?.peer"> → {{ task.peer.name }}</span>
                    </div>
                </div>
                <v-chip :color="statusColor" size="small" variant="flat">
                    {{ progressPercent }}%
                </v-chip>
            </div>

            <!-- 进度条 -->
            <v-progress-linear
                :model-value="progressPercent"
                :color="statusColor"
                height="8"
                rounded
                class="mb-4"
            />

            <!-- 传输详情 -->
            <v-row dense>
                <v-col cols="6">
                    <div class="text-body-2 text-grey">已传输</div>
                    <div class="text-subtitle-2">
                        {{ formatSize(task?.transferredBytes || 0) }} /
                        {{ formatSize(task?.file.size || 0) }}
                    </div>
                </v-col>
                <v-col cols="6">
                    <div class="text-body-2 text-grey">传输速度</div>
                    <div class="text-subtitle-2">
                        {{ formatSpeed(task?.speed || 0) }}
                    </div>
                </v-col>
                <v-col cols="6">
                    <div class="text-body-2 text-grey">预估剩余时间</div>
                    <div class="text-subtitle-2">
                        {{ formatTime(task?.estimatedTimeRemaining) }}
                    </div>
                </v-col>
                <v-col cols="6">
                    <div class="text-body-2 text-grey">传输模式</div>
                    <div class="text-subtitle-2">
                        {{ task?.mode === 'local' ? '本地网络' : '云盘中转' }}
                    </div>
                </v-col>
            </v-row>

            <!-- 错误信息 -->
            <v-alert
                v-if="task?.error"
                type="error"
                variant="tonal"
                class="mt-4"
                density="compact"
            >
                {{ task.error }}
            </v-alert>

            <!-- 操作按钮 -->
            <div class="d-flex justify-end mt-4 ga-2">
                <v-btn
                    v-if="task?.status === 'transferring'"
                    color="warning"
                    variant="outlined"
                    size="small"
                    @click="handleCancel"
                >
                    取消传输
                </v-btn>
                <v-btn
                    v-if="task?.status === 'failed'"
                    color="error"
                    variant="outlined"
                    size="small"
                    @click="handleRetry"
                >
                    重试
                </v-btn>
                <v-btn
                    v-if="
                        ['completed', 'failed', 'cancelled'].includes(
                            task?.status || ''
                        )
                    "
                    color="primary"
                    variant="outlined"
                    size="small"
                    @click="handleRemove"
                >
                    移除
                </v-btn>
            </div>
        </v-card-text>
    </v-card>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { TransferTask } from '../../types'
import { getStatusText, formatSpeed, formatTimeRemaining } from '../../types'
import { formatFileSize } from '../../types/file'

const props = defineProps<{
    task: TransferTask | null
}>()

const emit = defineEmits<{
    (e: 'cancel', taskId: string): void
    (e: 'retry', task: TransferTask): void
    (e: 'remove', taskId: string): void
}>()

const progressPercent = computed(() => {
    return Math.round(props.task?.progress || 0)
})

const statusColor = computed(() => {
    const colors: Record<string, string> = {
        pending: 'grey',
        transferring: 'primary',
        completed: 'success',
        failed: 'error',
        cancelled: 'warning',
    }
    return colors[props.task?.status || 'pending']
})

const statusIcon = computed(() => {
    const icons: Record<string, string> = {
        pending: 'mdi-clock-outline',
        transferring: 'mdi-sync mdi-spin',
        completed: 'mdi-check-circle',
        failed: 'mdi-alert-circle',
        cancelled: 'mdi-cancel',
    }
    return icons[props.task?.status || 'pending']
})

function formatSize(bytes: number): string {
    return formatFileSize(bytes)
}

function formatTime(seconds?: number): string {
    return formatTimeRemaining(seconds)
}

function handleCancel() {
    if (props.task) {
        emit('cancel', props.task.id)
    }
}

function handleRetry() {
    if (props.task) {
        emit('retry', props.task)
    }
}

function handleRemove() {
    if (props.task) {
        emit('remove', props.task.id)
    }
}
</script>

<style scoped>
.progress-display {
    border-left: 4px solid rgb(var(--v-theme-primary));
}
</style>
