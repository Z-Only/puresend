<!-- 文件缩略图组件 -->
<template>
    <div class="file-thumbnail">
        <!-- 媒体文件缩略图 -->
        <template v-if="isMedia && thumbnailLoaded">
            <img
                v-if="isImage"
                :src="thumbnailSrc"
                :alt="name"
                class="thumbnail-image"
                @error="handleThumbnailError"
            />
            <video
                v-else-if="isVideo"
                :src="thumbnailSrc"
                class="thumbnail-video"
                @loadeddata="handleVideoLoaded"
                @error="handleThumbnailError"
            />
            <div v-else-if="isAudio" class="thumbnail-audio">
                <v-icon :icon="mdiMusic" :size="iconSize" color="white" />
            </div>
        </template>

        <!-- 加载中状态 -->
        <template v-else-if="isMedia && loading">
            <v-progress-circular
                indeterminate
                :size="size * 0.5"
                :width="2"
                color="primary"
            />
        </template>

        <!-- 加载失败或非媒体文件：显示文件类型图标 -->
        <template v-else>
            <FileTypeIcon :mime-type="mimeType" :size="size" />
        </template>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { readFile } from '@tauri-apps/plugin-fs'
import { mdiMusic } from '@mdi/js'
import FileTypeIcon from './FileTypeIcon.vue'
import type { ThumbnailInfo } from '../../types/content'
import { DEFAULT_THUMBNAIL_CONFIG } from '../../types/content'

const props = withDefaults(
    defineProps<{
        /** 文件路径 */
        path: string
        /** 文件名 */
        name: string
        /** MIME 类型 */
        mimeType: string
        /** 是否为媒体文件 */
        isMedia: boolean
        /** 缩略图信息 */
        thumbnail?: ThumbnailInfo
        /** 尺寸 */
        size?: number
    }>(),
    {
        size: 64,
    }
)

const emit = defineEmits<{
    /** 缩略图加载完成 */
    (e: 'loaded', thumbnail: ThumbnailInfo): void
    /** 缩略图加载失败 */
    (e: 'error', error: string): void
}>()

const loading = ref(true)
const thumbnailLoaded = ref(false)
const thumbnailError = ref<string | null>(null)
const base64Data = ref<string | null>(null)

const iconSize = computed(() => Math.round(props.size * 0.5))

const isImage = computed(
    () => props.mimeType.startsWith('image/') && !props.mimeType.includes('svg')
)

const isVideo = computed(() => props.mimeType.startsWith('video/'))

const isAudio = computed(() => props.mimeType.startsWith('audio/'))

const thumbnailSrc = computed(() => {
    // 如果已有缩略图路径（base64 或 URL），直接使用
    if (props.thumbnail?.path) {
        return props.thumbnail.path
    }

    // 使用加载的 base64 数据
    if (base64Data.value) {
        return base64Data.value
    }

    return ''
})

/**
 * 将文件数据转换为 Blob URL
 */
function uint8ArrayToBlobUrl(data: Uint8Array, mimeType: string): string {
    // 创建一个新的 ArrayBuffer 副本，避免 SharedArrayBuffer 类型问题
    const arrayBuffer = new ArrayBuffer(data.length)
    const view = new Uint8Array(arrayBuffer)
    view.set(data)
    const blob = new Blob([arrayBuffer], { type: mimeType })
    return URL.createObjectURL(blob)
}

/**
 * 加载缩略图
 */
async function loadThumbnail() {
    if (!props.isMedia || !props.path) {
        loading.value = false
        return
    }

    // 如果已经有缩略图，直接使用
    if (props.thumbnail?.loaded) {
        thumbnailLoaded.value = true
        loading.value = false
        return
    }

    loading.value = true
    thumbnailError.value = null

    try {
        // 对于图片和视频，读取文件并转换为 blob URL
        if (isImage.value || isVideo.value) {
            const fileData = await readFile(props.path)
            base64Data.value = uint8ArrayToBlobUrl(fileData, props.mimeType)
            thumbnailLoaded.value = true
            emit('loaded', {
                path: base64Data.value,
                size: DEFAULT_THUMBNAIL_CONFIG.small,
                loaded: true,
            })
        } else {
            // 音频文件不显示缩略图，使用图标
            thumbnailLoaded.value = false
        }
    } catch (error) {
        const errorMsg =
            error instanceof Error ? error.message : 'Unknown error'
        thumbnailError.value = errorMsg
        emit('error', errorMsg)
    } finally {
        loading.value = false
    }
}

function handleThumbnailError() {
    thumbnailLoaded.value = false
    thumbnailError.value = 'Failed to load thumbnail'
    emit('error', 'Failed to load thumbnail')
}

function handleVideoLoaded() {
    thumbnailLoaded.value = true
}

// 监听路径变化重新加载
watch(
    () => props.path,
    () => {
        thumbnailLoaded.value = false
        thumbnailError.value = null
        base64Data.value = null
        loadThumbnail()
    }
)

onMounted(() => {
    loadThumbnail()
})
</script>

<style scoped>
.file-thumbnail {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    border-radius: 8px;
    background-color: rgba(0, 0, 0, 0.05);
}

.thumbnail-image,
.thumbnail-video {
    width: 100%;
    height: 100%;
    object-fit: cover;
}

.thumbnail-audio {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    border-radius: 8px;
}
</style>
