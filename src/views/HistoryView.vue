<!-- 历史记录页面 -->
<template>
    <v-container fluid class="history-view">
        <v-card>
            <v-card-title class="d-flex align-center justify-space-between">
                <span>{{ t('history.title') }}</span>
                <div class="d-flex align-center ga-2">
                    <span
                        v-if="transferStore.historyCount > 0"
                        class="text-body-2 text-grey"
                    >
                        {{
                            t('history.recordCount', {
                                count: transferStore.historyCount,
                            })
                        }}
                    </span>
                    <v-btn
                        v-if="transferStore.selectedHistoryCount > 0"
                        color="error"
                        variant="text"
                        size="small"
                        @click="handleBatchDelete"
                    >
                        {{
                            t('history.deleteSelected', {
                                count: transferStore.selectedHistoryCount,
                            })
                        }}
                    </v-btn>
                    <v-btn
                        v-if="transferStore.historyCount > 0"
                        color="error"
                        variant="text"
                        size="small"
                        @click="showClearDialog = true"
                    >
                        {{ t('history.clear') }}
                    </v-btn>
                </div>
            </v-card-title>

            <v-divider />

            <!-- 筛选和排序 -->
            <v-card-text v-if="transferStore.historyCount > 0" class="pb-0">
                <v-row dense>
                    <v-col cols="6" sm="3">
                        <v-select
                            v-model="filterDirection"
                            :items="directionOptions"
                            :label="t('history.filter.direction')"
                            density="compact"
                            variant="outlined"
                            hide-details
                        />
                    </v-col>
                    <v-col cols="6" sm="3">
                        <v-select
                            v-model="filterStatus"
                            :items="statusOptions"
                            :label="t('history.filter.status')"
                            density="compact"
                            variant="outlined"
                            hide-details
                        />
                    </v-col>
                    <v-col cols="6" sm="3">
                        <v-select
                            v-model="sortField"
                            :items="sortFieldOptions"
                            :label="t('history.sort.field')"
                            density="compact"
                            variant="outlined"
                            hide-details
                        />
                    </v-col>
                    <v-col cols="6" sm="3">
                        <v-select
                            v-model="sortOrder"
                            :items="sortOrderOptions"
                            :label="t('history.sort.order')"
                            density="compact"
                            variant="outlined"
                            hide-details
                        />
                    </v-col>
                </v-row>
            </v-card-text>

            <v-divider v-if="transferStore.historyCount > 0" class="mt-2" />

            <!-- 空状态 -->
            <v-card-text
                v-if="transferStore.historyCount === 0"
                class="d-flex flex-column align-center justify-center py-16"
            >
                <v-icon
                    :icon="mdiHistory"
                    size="64"
                    color="grey"
                    class="mb-4"
                />
                <div class="text-h6 text-grey">{{ t('history.empty') }}</div>
                <div class="text-body-2 text-grey">
                    {{ t('history.emptyHint') }}
                </div>
            </v-card-text>

            <!-- 无筛选结果 -->
            <v-card-text
                v-else-if="transferStore.filteredHistory.length === 0"
                class="d-flex flex-column align-center justify-center py-16"
            >
                <v-icon
                    :icon="mdiFilterRemove"
                    size="64"
                    color="grey"
                    class="mb-4"
                />
                <div class="text-h6 text-grey">
                    {{ t('history.noResults') }}
                </div>
            </v-card-text>

            <!-- 历史列表 -->
            <v-list v-else density="compact">
                <!-- 全选 -->
                <v-list-item>
                    <v-checkbox
                        :model-value="isAllSelected"
                        :indeterminate="isSomeSelected && !isAllSelected"
                        :label="t('history.selectAll')"
                        density="compact"
                        hide-details
                        @update:model-value="
                            (v: boolean | null) =>
                                toggleAllSelection(v ?? false)
                        "
                    />
                </v-list-item>

                <v-divider />

                <template
                    v-for="(item, index) in transferStore.filteredHistory"
                    :key="item.id"
                >
                    <v-list-item>
                        <template #prepend>
                            <v-checkbox
                                :model-value="item.selected"
                                density="compact"
                                hide-details
                                class="mr-2"
                                @update:model-value="toggleSelection(item.id)"
                            />
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

                        <v-list-item-title>
                            {{ displayFileName(item) }}
                        </v-list-item-title>
                        <v-list-item-subtitle>
                            <span class="mr-2">{{
                                formatSize(item.fileSize)
                            }}</span>
                            <span class="mr-2">•</span>
                            <span class="mr-2">{{
                                displayPeerName(item)
                            }}</span>
                            <span class="mr-2">•</span>
                            <span>{{ formatTime(item.completedAt) }}</span>
                        </v-list-item-subtitle>

                        <template #append>
                            <div class="d-flex align-center ga-2">
                                <v-chip
                                    :color="getStatusColor(item.status)"
                                    size="small"
                                    variant="flat"
                                >
                                    {{ t(getStatusKey(item.status)) }}
                                </v-chip>
                                <v-btn
                                    icon
                                    size="small"
                                    variant="text"
                                    color="error"
                                    @click="showDeleteDialog(item)"
                                >
                                    <v-icon :icon="mdiDelete" size="18" />
                                </v-btn>
                            </div>
                        </template>
                    </v-list-item>

                    <v-divider
                        v-if="index < transferStore.filteredHistory.length - 1"
                    />
                </template>
            </v-list>
        </v-card>

        <!-- 删除确认对话框 -->
        <v-dialog v-model="deleteDialog" max-width="400">
            <v-card>
                <v-card-title>{{
                    t('history.deleteConfirm.title')
                }}</v-card-title>
                <v-card-text>
                    {{
                        t('history.deleteConfirm.message', {
                            name: itemToDelete?.fileName,
                        })
                    }}
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn variant="text" @click="deleteDialog = false">
                        {{ t('common.cancel') }}
                    </v-btn>
                    <v-btn color="error" variant="flat" @click="confirmDelete">
                        {{ t('common.delete') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

        <!-- 批量删除确认对话框 -->
        <v-dialog v-model="batchDeleteDialog" max-width="400">
            <v-card>
                <v-card-title>{{
                    t('history.batchDeleteConfirm.title')
                }}</v-card-title>
                <v-card-text>
                    {{
                        t('history.batchDeleteConfirm.message', {
                            count: transferStore.selectedHistoryCount,
                        })
                    }}
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn variant="text" @click="batchDeleteDialog = false">
                        {{ t('common.cancel') }}
                    </v-btn>
                    <v-btn
                        color="error"
                        variant="flat"
                        @click="confirmBatchDelete"
                    >
                        {{ t('common.delete') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

        <!-- 清空全部确认对话框 -->
        <v-dialog v-model="showClearDialog" max-width="400">
            <v-card>
                <v-card-title>{{
                    t('history.clearConfirm.title')
                }}</v-card-title>
                <v-card-text>
                    {{ t('history.clearConfirm.message') }}
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn variant="text" @click="showClearDialog = false">
                        {{ t('common.cancel') }}
                    </v-btn>
                    <v-btn color="error" variant="flat" @click="handleClearAll">
                        {{ t('history.clear') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>
    </v-container>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
    mdiHistory,
    mdiArrowUp,
    mdiArrowDown,
    mdiDelete,
    mdiFilterRemove,
} from '@mdi/js'
import { useTransferStore } from '@/stores/transfer'
import { useSettingsStore } from '@/stores/settings'
import { getStatusKey, formatFileSize } from '@/types'
import type {
    TransferHistoryItem,
    HistorySortField,
    HistorySortOrder,
} from '@/types'
import type { TaskStatus, TransferDirection } from '@/types'

const { t } = useI18n()
const transferStore = useTransferStore()
const settingsStore = useSettingsStore()

// 筛选状态
const filterDirection = ref<TransferDirection | 'all'>('all')
const filterStatus = ref<TaskStatus | 'all'>('all')

// 排序状态
const sortField = ref<HistorySortField>('completedAt')
const sortOrder = ref<HistorySortOrder>('desc')

// 对话框状态
const deleteDialog = ref(false)
const batchDeleteDialog = ref(false)
const showClearDialog = ref(false)
const itemToDelete = ref<TransferHistoryItem | null>(null)

// 筛选选项
const directionOptions = computed(() => [
    { title: t('history.filter.all'), value: 'all' },
    { title: t('history.direction.send'), value: 'send' },
    { title: t('history.direction.receive'), value: 'receive' },
])

const statusOptions = computed(() => [
    { title: t('history.filter.all'), value: 'all' },
    { title: t('transfer.status.completed'), value: 'completed' },
    { title: t('transfer.status.failed'), value: 'failed' },
    { title: t('transfer.status.cancelled'), value: 'cancelled' },
])

const sortFieldOptions = computed(() => [
    { title: t('history.sort.fields.time'), value: 'completedAt' },
    { title: t('history.sort.fields.fileName'), value: 'fileName' },
    { title: t('history.sort.fields.fileSize'), value: 'fileSize' },
])

const sortOrderOptions = computed(() => [
    { title: t('history.sort.orders.desc'), value: 'desc' },
    { title: t('history.sort.orders.asc'), value: 'asc' },
])

// 全选状态
const isAllSelected = computed(() => {
    const count = transferStore.filteredHistory.length
    return count > 0 && transferStore.selectedHistoryCount === count
})

const isSomeSelected = computed(() => transferStore.selectedHistoryCount > 0)

// 监听筛选和排序变化
watch([filterDirection, filterStatus], ([direction, status]) => {
    transferStore.setHistoryFilter({ direction, status })
})

watch([sortField, sortOrder], ([field, order]) => {
    transferStore.setHistorySort({ field, order })
})

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

// 隐私模式显示处理
function displayFileName(item: TransferHistoryItem): string {
    const privacy = settingsStore.history.privacy
    if (privacy.enabled && privacy.hideFileName) {
        return t('history.privacy.hiddenFileName')
    }
    return item.fileName
}

function displayPeerName(item: TransferHistoryItem): string {
    const privacy = settingsStore.history.privacy
    if (privacy.enabled && privacy.hidePeerName) {
        return t('history.privacy.hiddenPeerName')
    }
    return item.peerName || t('history.unknownDevice')
}

// 选择操作
function toggleSelection(id: string) {
    transferStore.toggleHistorySelection(id)
}

function toggleAllSelection(selected: boolean) {
    transferStore.toggleAllHistorySelection(selected)
}

// 删除操作
function showDeleteDialog(item: TransferHistoryItem) {
    itemToDelete.value = item
    deleteDialog.value = true
}

async function confirmDelete() {
    if (itemToDelete.value) {
        await transferStore.removeHistoryItem(itemToDelete.value.id)
        itemToDelete.value = null
    }
    deleteDialog.value = false
}

function handleBatchDelete() {
    batchDeleteDialog.value = true
}

async function confirmBatchDelete() {
    const selectedIds = transferStore.selectedHistoryItems.map(
        (item) => item.id
    )
    await transferStore.removeHistoryItems(selectedIds)
    batchDeleteDialog.value = false
}

async function handleClearAll() {
    await transferStore.clearHistory()
    showClearDialog.value = false
}

onMounted(async () => {
    // 加载历史记录
    if (!transferStore.historyLoaded) {
        await transferStore.loadHistory()
    }

    // 执行自动清理
    const autoCleanup = settingsStore.history.autoCleanup
    if (autoCleanup.strategy !== 'disabled') {
        await transferStore.applyAutoCleanup(autoCleanup.strategy, {
            retentionDays: autoCleanup.retentionDays,
            maxCount: autoCleanup.maxCount,
        })
    }
})
</script>

<style scoped>
.history-view {
    max-width: 800px;
    margin: 0 auto;
}
</style>
