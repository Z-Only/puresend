<template>
    <v-dialog v-model="dialogVisible" max-width="800">
        <v-card>
            <v-card-title>
                <span class="text-h6">{{
                    t('cloudTransfer.directoryBrowser.title')
                }}</span>
            </v-card-title>

            <v-card-text class="browser-card-text">
                <!-- 目录面包屑导航 -->
                <div class="directory-breadcrumb mb-3">
                    <v-breadcrumbs :items="breadcrumbItems">
                        <template #divider>
                            <v-icon :icon="mdiChevronRight" />
                        </template>
                        <template #item="{ item }">
                            <v-breadcrumbs-item
                                :disabled="(item as BreadcrumbNavItem).disabled"
                                @click="
                                    navigateToPath(
                                        (item as BreadcrumbNavItem).path
                                    )
                                "
                            >
                                <v-icon
                                    v-if="(item as BreadcrumbNavItem).isRoot"
                                    :icon="mdiFolder"
                                    class="mr-1"
                                />
                                {{ item.title }}
                            </v-breadcrumbs-item>
                        </template>
                    </v-breadcrumbs>
                </div>

                <!-- 操作栏 -->
                <div class="d-flex align-center justify-end mb-3 pr-2">
                    <!-- 新建文件夹按钮 -->
                    <v-btn
                        :prepend-icon="mdiFolderPlus"
                        variant="tonal"
                        size="small"
                        @click="showNewFolderDialog = true"
                    >
                        {{ t('cloudTransfer.directoryBrowser.newFolder') }}
                    </v-btn>
                </div>

                <!-- 加载状态 -->
                <div v-if="loading" class="d-flex justify-center pa-8">
                    <v-progress-circular indeterminate color="primary" />
                </div>

                <!-- 错误提示 -->
                <v-alert
                    v-else-if="error"
                    type="error"
                    variant="tonal"
                    density="compact"
                    class="mb-3"
                >
                    {{ error }}
                </v-alert>

                <!-- 文件列表 -->
                <v-list
                    v-else-if="items.length > 0"
                    class="file-list custom-scrollbar pa-0"
                >
                    <v-list-item
                        v-for="item in sortedItems"
                        :key="item.path"
                        class="file-item"
                        :class="{ 'selected-item': selectedPath === item.path }"
                        density="compact"
                        @click="handleItemClick(item)"
                    >
                        <template v-slot:prepend>
                            <v-icon
                                :icon="
                                    getFileTypeIcon(item.name, item.isDirectory)
                                "
                                :color="item.isDirectory ? 'warning' : 'info'"
                                size="24"
                                class="mr-2"
                            />
                        </template>
                        <template v-slot:title>
                            <div class="file-title-row">
                                <span class="file-name text-truncate">{{
                                    item.name
                                }}</span>
                                <span class="file-meta">
                                    <span class="file-size">
                                        {{
                                            formatFileSizeSafe(
                                                item.size,
                                                item.isDirectory
                                            )
                                        }}
                                    </span>
                                    <span
                                        v-if="item.modified"
                                        class="file-time"
                                    >
                                        {{ formatTime(item.modified) }}
                                    </span>
                                </span>
                            </div>
                        </template>
                    </v-list-item>
                </v-list>

                <!-- 空目录提示 -->
                <div
                    v-else
                    class="d-flex flex-column align-center justify-center pa-8"
                >
                    <v-icon
                        :icon="mdiFolderOpen"
                        size="64"
                        color="grey"
                        class="mb-4"
                    />
                    <div class="text-body-1 text-grey">
                        {{ t('cloudTransfer.directoryBrowser.empty') }}
                    </div>
                </div>
            </v-card-text>

            <v-card-actions>
                <v-btn variant="text" @click="handleCancel">
                    {{ t('common.cancel') }}
                </v-btn>
                <v-btn
                    v-if="mode === 'directory'"
                    color="primary"
                    variant="flat"
                    @click="handleSelectCurrentDirectory"
                >
                    {{ t('common.confirm') }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>

    <!-- 新建文件夹对话框 -->
    <v-dialog v-model="showNewFolderDialog" max-width="400">
        <v-card>
            <v-card-title>
                {{ t('cloudTransfer.directoryBrowser.newFolder') }}
            </v-card-title>
            <v-card-text>
                <v-text-field
                    v-model="newFolderName"
                    :label="t('cloudTransfer.directoryBrowser.newFolderName')"
                    :placeholder="
                        t('cloudTransfer.directoryBrowser.newFolderPlaceholder')
                    "
                    autofocus
                    @keyup.enter="handleCreateFolder"
                />
            </v-card-text>
            <v-card-actions>
                <v-spacer />
                <v-btn variant="text" @click="showNewFolderDialog = false">
                    {{ t('common.cancel') }}
                </v-btn>
                <v-btn
                    color="primary"
                    variant="flat"
                    @click="handleCreateFolder"
                >
                    {{ t('common.confirm') }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useCloudStore } from '@/stores/cloud'
import { formatFileSizeSafe, formatTime, getFileTypeIcon } from '@/utils/format'
import type { CloudFileItem } from '@/types/cloud'
import {
    mdiFolder,
    mdiChevronRight,
    mdiFolderPlus,
    mdiFolderOpen,
} from '@mdi/js'

interface BreadcrumbNavItem {
    title: string
    path: string
    disabled: boolean
    isRoot: boolean
}

interface Props {
    modelValue: boolean
    accountId: string
    initialPath?: string
    mode?: 'directory' | 'file'
}

interface Emits {
    (e: 'update:modelValue', value: boolean): void
    (e: 'select', path: string): void
}

const props = withDefaults(defineProps<Props>(), {
    initialPath: '/',
    mode: 'directory',
})

const emit = defineEmits<Emits>()

const { t } = useI18n()
const cloudStore = useCloudStore()

const currentPath = ref(props.initialPath)
const items = ref<CloudFileItem[]>([])
const loading = ref(false)
const error = ref('')
const selectedPath = ref('')
const showNewFolderDialog = ref(false)
const newFolderName = ref('')

const dialogVisible = computed({
    get: () => props.modelValue,
    set: (value: boolean) => emit('update:modelValue', value),
})

const breadcrumbItems = computed<BreadcrumbNavItem[]>(() => {
    if (!currentPath.value) return []

    const parts = currentPath.value.split('/').filter(Boolean)
    const items: BreadcrumbNavItem[] = [
        {
            title: t('cloudTransfer.directoryBrowser.currentPath'),
            path: '/',
            disabled: currentPath.value === '/',
            isRoot: true,
        },
    ]

    let pathAccumulator = ''
    parts.forEach((part, index) => {
        pathAccumulator += `/${part}`
        items.push({
            title: part,
            path: pathAccumulator,
            disabled: index === parts.length - 1,
            isRoot: false,
        })
    })

    return items
})

function navigateToPath(path: string): void {
    if (path !== currentPath.value) {
        currentPath.value = path
        loadDirectory()
    }
}

const sortedItems = computed(() => {
    return [...items.value].sort((a, b) => {
        if (a.isDirectory && !b.isDirectory) return -1
        if (!a.isDirectory && b.isDirectory) return 1
        return a.name.localeCompare(b.name)
    })
})

async function loadDirectory(): Promise<void> {
    if (!props.accountId) return

    loading.value = true
    error.value = ''
    items.value = []
    selectedPath.value = ''

    try {
        items.value = await cloudStore.browseDirectory(
            props.accountId,
            currentPath.value
        )
    } catch (err) {
        error.value = t('cloudTransfer.directoryBrowser.loadError')
        console.error('[CloudDirectoryBrowser] 加载目录失败:', err)
    } finally {
        loading.value = false
    }
}

function handleItemClick(item: CloudFileItem): void {
    if (item.isDirectory) {
        currentPath.value = item.path
        loadDirectory()
    } else if (props.mode === 'file') {
        selectedPath.value = item.path
    }
}

function handleSelectCurrentDirectory(): void {
    emit('select', currentPath.value)
    dialogVisible.value = false
}

async function handleCreateFolder(): Promise<void> {
    if (!newFolderName.value.trim()) {
        return
    }

    const folderPath =
        currentPath.value === '/'
            ? '/' + newFolderName.value.trim()
            : currentPath.value + '/' + newFolderName.value.trim()

    try {
        await cloudStore.createDirectory(props.accountId, folderPath)
        showNewFolderDialog.value = false
        newFolderName.value = ''
        loadDirectory()
    } catch (err) {
        error.value = t('cloudTransfer.directoryBrowser.createFolderError')
        console.error('[CloudDirectoryBrowser] 创建文件夹失败:', err)
    }
}

function handleCancel(): void {
    dialogVisible.value = false
}

watch(
    () => props.modelValue,
    (visible) => {
        if (visible) {
            currentPath.value = props.initialPath
            loadDirectory()
        }
    }
)

watch(
    () => props.initialPath,
    (newPath) => {
        if (props.modelValue) {
            currentPath.value = newPath
            loadDirectory()
        }
    }
)
</script>

<style scoped>
.directory-breadcrumb {
    padding: 8px 0;
    background-color: rgba(var(--v-theme-surface-variant), 0.1);
    border-radius: 4px;
    padding-left: 8px;
    padding-right: 8px;
}

.directory-breadcrumb :deep(.v-breadcrumbs) {
    padding: 0;
}

.directory-breadcrumb
    :deep(.v-breadcrumbs-item:not(.v-breadcrumbs-item--disabled)) {
    cursor: pointer;
    color: rgb(var(--v-theme-primary));
}

.directory-breadcrumb
    :deep(.v-breadcrumbs-item:not(.v-breadcrumbs-item--disabled)):hover {
    text-decoration: underline;
}

.directory-breadcrumb :deep(.v-breadcrumbs-item--disabled) {
    cursor: default;
    opacity: 1;
}

.browser-card-text {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    max-height: 60vh;
}

.file-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
}

.file-item {
    cursor: pointer;
    transition: background-color 0.2s;
}

.file-item:hover {
    background-color: rgba(var(--v-theme-surface-variant), 0.08);
}

.file-item.selected-item {
    background-color: rgba(var(--v-theme-primary), 0.08);
}

.file-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
}

.file-name {
    font-weight: 500;
    flex: 1;
    min-width: 0;
}

.file-meta {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
}

.file-size {
    display: inline-block;
    min-width: 80px;
    text-align: left;
    color: rgba(var(--v-theme-on-surface), 0.6);
    font-size: 0.875rem;
    white-space: nowrap;
}

.file-time {
    display: inline-block;
    min-width: 160px;
    text-align: left;
    color: rgba(var(--v-theme-on-surface), 0.6);
    font-size: 0.875rem;
    white-space: nowrap;
}

.current-path-hint {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 400px;
}
</style>
