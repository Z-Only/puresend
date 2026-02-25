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
                    :class="{ 'mdi-spin': isSpinning }"
                    class="mr-3"
                />
                <div class="flex-grow-1 overflow-hidden">
                    <div class="text-subtitle-1 text-truncate">
                        {{
                            task?.file.name ||
                            t('transfer.progress.unknownFile')
                        }}
                    </div>
                    <div class="text-body-2 text-grey">
                        {{ t(getStatusKey(task?.status || 'pending')) }}
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
                    <div class="text-body-2 text-grey">
                        {{ t('transfer.progress.transferred') }}
                    </div>
                    <div class="text-subtitle-2">
                        {{ formatSize(task?.transferredBytes || 0) }} /
                        {{ formatSize(task?.file.size || 0) }}
                    </div>
                </v-col>
                <v-col cols="6">
                    <div class="text-body-2 text-grey">
                        {{ t('transfer.progress.speed') }}
                    </div>
                    <div class="text-subtitle-2">
                        {{ formatSpeed(task?.speed || 0) }}
                    </div>
                </v-col>
                <v-col cols="6">
                    <div class="text-body-2 text-grey">
                        {{ t('transfer.progress.timeRemaining') }}
                    </div>
                    <div class="text-subtitle-2">
                        {{ formatTime(task?.estimatedTimeRemaining) }}
                    </div>
                </v-col>
                <v-col cols="6">
                    <div class="text-body-2 text-grey">
                        {{ t('transfer.progress.mode') }}
                    </div>
                    <div class="text-subtitle-2">
                        {{
                            task?.mode === 'local'
                                ? t('transfer.progress.localNetwork')
                                : t('transfer.progress.cloudTransfer')
                        }}
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
                    {{ t('transfer.progress.cancelTransfer') }}
                </v-btn>
                <v-btn
                    v-if="task?.status === 'failed'"
                    color="error"
                    variant="outlined"
                    size="small"
                    @click="handleRetry"
                >
                    {{ t('transfer.progress.retry') }}
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
                    {{ t('transfer.progress.remove') }}
                </v-btn>
            </div>
        </v-card-text>
    </v-card>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { TransferTask } from '../../types'
import { getStatusKey, formatSpeed } from '../../types'
import { formatFileSize } from '../../types/file'
import {
    mdiClockOutline,
    mdiSync,
    mdiCheckCircle,
    mdiAlertCircle,
    mdiCancel,
} from '@mdi/js'

const { t } = useI18n()

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
        pending: mdiClockOutline,
        transferring: mdiSync,
        completed: mdiCheckCircle,
        failed: mdiAlertCircle,
        cancelled: mdiCancel,
    }
    return icons[props.task?.status || 'pending']
})

const isSpinning = computed(() => {
    return props.task?.status === 'transferring'
})

function formatSize(bytes: number): string {
    return formatFileSize(bytes)
}

function formatTime(seconds?: number): string {
    if (!seconds) return '--'
    if (seconds < 60) {
        return t('transfer.time.seconds', { count: seconds })
    } else if (seconds < 3600) {
        const minutes = Math.floor(seconds / 60)
        const secs = seconds % 60
        return t('transfer.time.minutesSeconds', { minutes, seconds: secs })
    } else {
        const hours = Math.floor(seconds / 3600)
        const minutes = Math.floor((seconds % 3600) / 60)
        return t('transfer.time.hoursMinutes', { hours, minutes })
    }
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

.mdi-spin {
    animation: mdi-spin 1s linear infinite;
}

@keyframes mdi-spin {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}
</style>
