<!-- 媒体文件选择器组件 -->
<template>
    <div class="media-picker">
        <v-card variant="outlined" class="pa-4">
            <v-card-text>
                <div class="d-flex align-center mb-3">
                    <v-icon
                        :icon="mdiImageMultiple"
                        class="mr-2"
                        color="primary"
                    />
                    <span class="text-subtitle-1 font-weight-bold"
                        >选择媒体文件</span
                    >
                </div>

                <div class="text-body-2 text-grey mb-3">
                    支持图片、视频、音频文件
                </div>

                <v-btn
                    color="primary"
                    :loading="loading"
                    block
                    class="text-center picker-btn"
                    @click="pickMedia"
                >
                    <v-icon :icon="mdiFolderOpen" />
                    <span class="btn-text">选择媒体文件</span>
                </v-btn>

                <v-alert
                    v-if="errorMessage"
                    type="error"
                    variant="tonal"
                    class="mt-4"
                    density="compact"
                >
                    {{ errorMessage }}
                </v-alert>

                <!-- 已选择的媒体文件预览 -->
                <div v-if="selectedFiles.length > 0" class="mt-4">
                    <v-divider class="mb-3" />
                    <div class="text-subtitle-2 mb-2">
                        已选择 {{ selectedFiles.length }} 个文件
                    </div>
                    <v-row>
                        <v-col
                            v-for="file in selectedFiles"
                            :key="file.path"
                            cols="6"
                            md="4"
                            lg="3"
                        >
                            <v-card
                                variant="outlined"
                                class="media-preview-card"
                            >
                                <v-img
                                    v-if="isImage(file.mimeType)"
                                    :src="file.path"
                                    height="100"
                                    cover
                                />
                                <v-icon
                                    v-else
                                    :icon="getMediaIcon(file.mimeType)"
                                    size="48"
                                    class="ma-4"
                                />
                                <v-card-text class="pa-2">
                                    <div class="text-caption text-truncate">
                                        {{ file.name }}
                                    </div>
                                </v-card-text>
                            </v-card>
                        </v-col>
                    </v-row>
                </div>
            </v-card-text>
        </v-card>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { getFileMetadata } from '../../services/transferService'
import type { ContentItem } from '../../types'
import {
    mdiImageMultiple,
    mdiFolderOpen,
    mdiFileImage,
    mdiFileVideo,
    mdiFileMusic,
    mdiFile,
} from '@mdi/js'

const emit = defineEmits<{
    (e: 'select', item: ContentItem): void
}>()

const loading = ref(false)
const selectedFiles = ref<ContentItem[]>([])
const errorMessage = ref('')

async function pickMedia() {
    loading.value = true
    errorMessage.value = ''
    try {
        const selected = await open({
            multiple: true,
            filters: [
                {
                    name: '媒体文件',
                    extensions: [
                        'jpg',
                        'jpeg',
                        'png',
                        'gif',
                        'webp',
                        'bmp',
                        'mp4',
                        'mov',
                        'avi',
                        'mkv',
                        'webm',
                        'mp3',
                        'wav',
                        'flac',
                        'ogg',
                    ],
                },
            ],
        })

        if (selected && Array.isArray(selected)) {
            // 并行获取所有文件的元数据
            selectedFiles.value = await Promise.all(
                selected.map(async (path) => {
                    const name = path.split(/[/\\]/).pop() || path
                    const extension = name.split('.').pop()?.toLowerCase() || ''

                    let size = 0
                    let mimeType = getMimeType(extension)

                    try {
                        const metadata = await getFileMetadata(path)
                        size = metadata.size
                        mimeType = metadata.mime_type || getMimeType(extension)
                    } catch (metaError) {
                        console.warn(`获取文件 ${name} 元数据失败：`, metaError)
                    }

                    return {
                        type: 'media' as const,
                        path,
                        name,
                        size,
                        mimeType,
                        createdAt: Date.now(),
                    }
                })
            )

            // 如果有文件，发送第一个（后续可改为多选）
            if (selectedFiles.value.length > 0) {
                emit('select', selectedFiles.value[0])
            }
        }
        // 用户取消选择时不显示错误信息，静默关闭即可
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)
        errorMessage.value = `选择媒体文件失败：${errorMsg}`
        console.error('选择媒体文件失败:', error)
    } finally {
        loading.value = false
    }
}

function isImage(mimeType: string): boolean {
    return mimeType.startsWith('image/')
}

function getMediaIcon(mimeType: string) {
    if (mimeType.startsWith('image/')) return mdiFileImage
    if (mimeType.startsWith('video/')) return mdiFileVideo
    if (mimeType.startsWith('audio/')) return mdiFileMusic
    return mdiFile
}

function getMimeType(extension: string): string {
    const mimeTypes: Record<string, string> = {
        // 图片
        jpg: 'image/jpeg',
        jpeg: 'image/jpeg',
        png: 'image/png',
        gif: 'image/gif',
        webp: 'image/webp',
        bmp: 'image/bmp',
        // 视频
        mp4: 'video/mp4',
        mov: 'video/quicktime',
        avi: 'video/x-msvideo',
        mkv: 'video/x-matroska',
        webm: 'video/webm',
        // 音频
        mp3: 'audio/mpeg',
        wav: 'audio/wav',
        flac: 'audio/flac',
        ogg: 'audio/ogg',
    }
    return mimeTypes[extension] || 'application/octet-stream'
}
</script>

<style scoped>
.media-preview-card {
    height: 100%;
}

.picker-btn {
    display: grid !important;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    justify-items: center;
}

.picker-btn .v-icon {
    grid-column: 1;
}

.picker-btn .btn-text {
    grid-column: 2;
    text-align: center;
}
</style>
