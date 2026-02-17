<!-- 文件选择组件 - 支持拖拽和点击选择 -->
<template>
    <v-card
        class="file-selector"
        :class="{ 'file-selector--dragover': isDragover }"
        @dragover.prevent="handleDragOver"
        @dragleave.prevent="handleDragLeave"
        @drop.prevent="handleDrop"
    >
        <v-card-text
            class="d-flex flex-column align-center justify-center pa-8"
        >
            <v-icon
                :icon="isDragover ? mdiCloudUpload : mdiFilePlus"
                size="64"
                :color="isDragover ? 'primary' : 'grey'"
                class="mb-4"
            />

            <div class="text-h6 mb-2">
                {{
                    isDragover
                        ? t('fileSelector.releaseToUpload')
                        : t('fileSelector.dragDropHint')
                }}
            </div>

            <div class="text-body-2 text-grey mb-4">
                {{ t('fileSelector.orClickToSelect') }}
            </div>

            <v-btn
                color="primary"
                variant="outlined"
                block
                class="text-center"
                :loading="loading"
                @click.stop="openFileDialog"
            >
                <v-icon :icon="mdiFolderOpen" class="mr-2" />
                {{ t('fileSelector.selectFile') }}
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
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { open } from '@tauri-apps/plugin-dialog'
import { inferMimeType } from '../../types'
import { getFileMetadata } from '../../services/transferService'
import { mdiCloudUpload, mdiFilePlus, mdiFolderOpen } from '@mdi/js'

const { t } = useI18n()

const emit = defineEmits<{
    (
        e: 'select',
        file: { path: string; name: string; size: number; type: string }
    ): void
    (e: 'clear'): void
}>()

const isDragover = ref(false)
const selectedFile = ref<{
    path: string
    name: string
    size: number
    type: string
} | null>(null)
const errorMessage = ref('')
const loading = ref(false)

function handleDragOver() {
    isDragover.value = true
}

function handleDragLeave() {
    isDragover.value = false
}

async function handleDrop(event: DragEvent) {
    isDragover.value = false
    errorMessage.value = ''

    // 从拖拽事件中获取文件
    try {
        const files = event.dataTransfer?.files
        if (files && files.length > 0) {
            const file = files[0]
            const name = file.name

            selectedFile.value = {
                path: file.name, // Web 环境下只能获取文件名
                name,
                size: file.size,
                type: inferMimeType(name),
            }

            emit('select', selectedFile.value)
        } else {
            errorMessage.value = t('fileSelector.noFileDetected')
        }
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)
        errorMessage.value = t('fileSelector.handleDragFailed', {
            error: errorMsg,
        })
        console.error('处理拖拽文件失败:', error)
    }
}

async function openFileDialog() {
    errorMessage.value = ''
    loading.value = true

    try {
        const selected = await open({
            multiple: false,
            title: t('fileSelector.selectFileToTransfer'),
        })

        if (selected && typeof selected === 'string') {
            const name = selected.split(/[/\\]/).pop() || selected

            // 获取文件元数据（包括真实大小）
            let metadata
            try {
                metadata = await getFileMetadata(selected)
            } catch (metaError) {
                // 如果获取元数据失败，使用默认值
                console.warn('获取文件元数据失败，使用默认值:', metaError)
            }

            const mimeType = metadata?.mimeType || inferMimeType(name)

            selectedFile.value = {
                path: selected,
                name,
                size: metadata?.size || 0,
                type: mimeType,
            }

            emit('select', selectedFile.value)
        }
        // 用户取消选择时不显示错误信息
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)
        errorMessage.value = t('fileSelector.openDialogFailed', {
            error: errorMsg,
        })
        console.error('打开文件对话框失败:', error)
    } finally {
        // 确保加载状态一定会重置
        loading.value = false
    }
}
</script>

<style scoped>
.file-selector {
    border: 2px dashed rgba(var(--v-theme-primary), 0.3);
    cursor: pointer;
    transition: all 0.2s ease;
    min-height: 200px;
}

.file-selector:hover {
    border-color: rgba(var(--v-theme-primary), 0.5);
    background: rgba(var(--v-theme-primary), 0.02);
}

.file-selector--dragover {
    border-color: rgb(var(--v-theme-primary));
    background: rgba(var(--v-theme-primary), 0.05);
}
</style>
