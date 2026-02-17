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
                    <span class="text-subtitle-1 font-weight-bold">
                        {{ t('mediaPicker.title') }}
                    </span>
                </div>

                <div class="text-body-2 text-grey mb-3">
                    {{ t('mediaPicker.description') }}
                </div>

                <v-btn
                    color="primary"
                    :loading="loading"
                    block
                    class="text-center picker-btn"
                    @click="pickMedia"
                >
                    <v-icon :icon="mdiFolderOpen" />
                    <span class="btn-text">{{
                        t('mediaPicker.selectMedia')
                    }}</span>
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
            </v-card-text>
        </v-card>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { open } from '@tauri-apps/plugin-dialog'
import { getFileMetadata } from '../../services/transferService'
import type { ContentItem } from '../../types'
import { getContentFilterNameKey } from '../../types'
import { mdiImageMultiple, mdiFolderOpen } from '@mdi/js'

const { t } = useI18n()

const emit = defineEmits<{
    (e: 'select', item: ContentItem): void
}>()

const loading = ref(false)
const errorMessage = ref('')

async function pickMedia() {
    loading.value = true
    errorMessage.value = ''
    try {
        const selected = await open({
            multiple: true,
            filters: [
                {
                    name: t(
                        getContentFilterNameKey('media') ||
                            'content.filter.media'
                    ),
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
            // 并行获取所有文件的元数据并逐个发送
            await Promise.all(
                selected.map(async (path) => {
                    const name = path.split(/[/\\]/).pop() || path
                    const extension = name.split('.').pop()?.toLowerCase() || ''

                    let size = 0
                    let mimeType = getMimeType(extension)

                    try {
                        const metadata = await getFileMetadata(path)
                        size = metadata.size
                        mimeType = metadata.mimeType || getMimeType(extension)
                    } catch (metaError) {
                        console.warn(`获取文件 ${name} 元数据失败：`, metaError)
                    }

                    emit('select', {
                        type: 'media' as const,
                        path,
                        name,
                        size,
                        mimeType,
                        createdAt: Date.now(),
                    })
                })
            )
        }
        // 用户取消选择时不显示错误信息，静默关闭即可
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)
        errorMessage.value = t('mediaPicker.selectFailed', { error: errorMsg })
        console.error('选择媒体文件失败:', error)
    } finally {
        loading.value = false
    }
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
