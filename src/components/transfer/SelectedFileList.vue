<!-- 已选文件列表组件 -->
<template>
    <v-card class="selected-file-list">
        <!-- 头部：统计信息和操作按钮 -->
        <v-card-title class="d-flex align-center justify-space-between pa-3">
            <div class="d-flex align-center">
                <v-icon :icon="mdiFileMultiple" class="mr-2" />
                <span>{{ t('selectedFiles.title') }}</span>
                <v-chip
                    v-if="stats.count > 0"
                    size="small"
                    color="primary"
                    class="ml-2"
                >
                    {{ t('selectedFiles.count', { count: stats.count }) }}
                </v-chip>
            </div>
            <div class="header-actions">
                <v-btn
                    v-if="files.length > 0"
                    variant="text"
                    color="error"
                    size="x-small"
                    @click="handleClearAll"
                >
                    {{ t('selectedFiles.clearAll') }}
                </v-btn>
            </div>
        </v-card-title>

        <!-- 文件统计信息 -->
        <v-card-subtitle v-if="files.length > 0" class="px-3 pb-2">
            {{
                t('selectedFiles.stats', {
                    count: stats.count,
                    size: stats.formattedSize,
                })
            }}
            <span v-if="stats.mediaCount > 0" class="ml-2">
                ({{
                    t('selectedFiles.mediaCount', { count: stats.mediaCount })
                }})
            </span>
        </v-card-subtitle>

        <!-- 空状态 -->
        <v-card-text v-if="files.length === 0" class="text-center py-8">
            <v-icon
                :icon="mdiInbox"
                size="64"
                color="grey-lighten-1"
                class="mb-4"
            />
            <div class="text-body-1 text-grey">
                {{ t('selectedFiles.empty') }}
            </div>
        </v-card-text>

        <!-- 文件列表 -->
        <v-card-text v-else class="file-list-container pa-2">
            <v-list class="file-list" bg-color="transparent">
                <v-list-item
                    v-for="file in files"
                    :key="file.id"
                    class="file-item mb-2"
                    rounded
                >
                    <template #prepend>
                        <div class="file-thumbnail-wrapper mr-3">
                            <FileThumbnail
                                :path="file.path"
                                :name="file.name"
                                :mime-type="file.mimeType"
                                :is-media="file.isMedia"
                                :thumbnail="file.thumbnail"
                                :size="48"
                                @loaded="
                                    (thumbnail) =>
                                        handleThumbnailLoaded(
                                            file.path,
                                            thumbnail
                                        )
                                "
                                @error="
                                    (err) =>
                                        handleThumbnailError(file.path, err)
                                "
                            />
                        </div>
                    </template>

                    <v-list-item-title class="text-truncate file-name">
                        {{ file.name }}
                    </v-list-item-title>

                    <v-list-item-subtitle
                        v-if="file.relativePath || file.isTemp"
                    >
                        <span v-if="file.relativePath" class="file-path">
                            {{ file.relativePath }}
                        </span>
                        <v-chip
                            v-if="file.isTemp"
                            size="x-small"
                            color="warning"
                            class="ml-2"
                        >
                            {{ t('selectedFiles.tempFile') }}
                        </v-chip>
                    </v-list-item-subtitle>

                    <template #append>
                        <div class="d-flex align-center">
                            <span class="file-size mr-2">{{
                                formatFileSize(file.size)
                            }}</span>
                            <v-btn
                                icon
                                variant="text"
                                size="small"
                                color="grey"
                                @click="handleRemove(file.path)"
                            >
                                <v-icon :icon="mdiClose" />
                            </v-btn>
                        </div>
                    </template>
                </v-list-item>
            </v-list>
        </v-card-text>

        <!-- 数量上限提示 -->
        <v-alert
            v-if="stats.isAtLimit"
            type="warning"
            variant="tonal"
            density="compact"
            class="ma-2"
        >
            {{ t('selectedFiles.limitReached', { limit: FILE_COUNT_LIMIT }) }}
        </v-alert>
    </v-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { mdiFileMultiple, mdiInbox, mdiClose } from '@mdi/js'
import FileThumbnail from './FileThumbnail.vue'
import type {
    SelectedFileItem,
    SelectedFilesStats,
    ThumbnailInfo,
} from '../../types/content'
import { FILE_COUNT_LIMIT } from '../../types/content'
import { formatFileSize } from '../../types/file'

defineProps<{
    /** 已选文件列表 */
    files: SelectedFileItem[]
    /** 统计信息 */
    stats: SelectedFilesStats
}>()

const emit = defineEmits<{
    /** 移除单个文件 */
    (e: 'remove', path: string): void
    /** 清空所有文件 */
    (e: 'clear'): void
    /** 缩略图加载完成 */
    (e: 'thumbnail-loaded', path: string, thumbnail: ThumbnailInfo): void
    /** 缩略图加载失败 */
    (e: 'thumbnail-error', path: string, error: string): void
}>()

const { t } = useI18n()

function handleRemove(path: string) {
    emit('remove', path)
}

function handleClearAll() {
    emit('clear')
}

function handleThumbnailLoaded(path: string, thumbnail: ThumbnailInfo) {
    emit('thumbnail-loaded', path, thumbnail)
}

function handleThumbnailError(path: string, error: string) {
    emit('thumbnail-error', path, error)
}
</script>

<style scoped>
.selected-file-list {
    max-height: 400px;
    display: flex;
    flex-direction: column;
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

.file-list-container {
    flex: 1;
    overflow-y: auto;
}

.file-list {
    padding: 0;
}

.file-item {
    border: 1px solid rgba(var(--v-border-color-rgb), 0.12);
    transition: all 0.2s ease;
    background-color: rgb(var(--v-theme-surface));
}

.file-item:hover {
    background-color: rgba(var(--v-primary-base), 0.04);
}

.file-thumbnail-wrapper {
    width: 48px;
    height: 48px;
    min-width: 48px;
    border-radius: 8px;
    overflow: hidden;
}

.file-name {
    color: rgb(var(--v-theme-on-surface));
    font-weight: 500;
}

.file-size {
    color: rgba(var(--v-theme-on-surface), var(--v-medium-emphasis-opacity));
    font-size: 0.875rem;
    white-space: nowrap;
    font-weight: 500;
}

.file-path {
    color: rgb(var(--v-theme-on-surface-variant));
    font-size: 0.75rem;
    opacity: 0.7;
}
</style>
