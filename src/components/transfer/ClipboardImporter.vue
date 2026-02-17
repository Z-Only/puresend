<!-- 剪贴板导入组件 -->
<template>
    <div class="clipboard-importer">
        <v-card variant="outlined" class="pa-4">
            <v-card-text>
                <div class="d-flex align-center mb-3">
                    <v-icon :icon="mdiClipboard" class="mr-2" color="primary" />
                    <span class="text-subtitle-1 font-weight-bold">
                        {{ t('clipboardImporter.title') }}
                    </span>
                </div>

                <div v-if="clipboardContent" class="mb-3">
                    <div class="text-body-2 text-grey mb-2">
                        {{ t('clipboardImporter.preview') }}：
                    </div>
                    <v-card variant="tonal" class="pa-3">
                        <div class="text-body-2 clipboard-preview">
                            {{ clipboardContent }}
                        </div>
                    </v-card>
                    <div class="text-caption text-grey mt-2">
                        {{
                            t('clipboardImporter.charCount', {
                                count: clipboardContent.length,
                            })
                        }}
                    </div>
                </div>

                <v-alert
                    v-if="errorMessage"
                    type="error"
                    variant="tonal"
                    class="mb-4"
                    density="compact"
                >
                    {{ errorMessage }}
                </v-alert>

                <div class="d-flex justify-center mt-4" style="gap: 8px">
                    <v-btn
                        color="primary"
                        :loading="loading"
                        min-width="120"
                        class="text-center"
                        @click="importFromClipboard"
                    >
                        {{
                            clipboardContent
                                ? t('clipboardImporter.reRead')
                                : t('clipboardImporter.readClipboard')
                        }}
                    </v-btn>

                    <v-btn
                        v-if="clipboardContent"
                        color="success"
                        min-width="120"
                        class="text-center"
                        @click="confirmImport"
                    >
                        {{ t('clipboardImporter.confirmImport') }}
                    </v-btn>
                </div>
            </v-card-text>
        </v-card>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { readText } from '@tauri-apps/plugin-clipboard-manager'
import { invoke } from '@tauri-apps/api/core'
import type { ContentItem } from '../../types'
import { mdiClipboard } from '@mdi/js'

const { t } = useI18n()

/** 带临时文件信息的内容项 */
interface ClipboardContentItem extends ContentItem {
    content?: string
    tempPath?: string
}

const emit = defineEmits<{
    (e: 'select', item: ClipboardContentItem): void
}>()

const loading = ref(false)
const clipboardContent = ref('')
const errorMessage = ref('')
const tempFilePath = ref<string | null>(null)

async function importFromClipboard() {
    loading.value = true
    errorMessage.value = ''
    try {
        const text = await readText()
        if (text) {
            clipboardContent.value = text
        } else {
            clipboardContent.value = ''
            errorMessage.value = t('clipboardImporter.clipboardEmpty')
        }
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)
        errorMessage.value = t('clipboardImporter.readFailed', {
            error: errorMsg,
        })
        console.error('读取剪贴板失败:', error)
    } finally {
        loading.value = false
    }
}

/**
 * 将剪贴板内容保存为临时文件
 */
async function saveToTempFile(content: string): Promise<string> {
    try {
        const tempPath = await invoke<string>('save_clipboard_to_temp', {
            content,
        })
        return tempPath
    } catch (error) {
        console.warn('保存临时文件失败，使用内存路径:', error)
        // 如果后端不支持，使用虚拟路径
        return `clipboard://temp/${Date.now()}.txt`
    }
}

async function confirmImport() {
    if (!clipboardContent.value) return

    loading.value = true
    try {
        // 将剪贴板内容保存为临时文本文件
        const tempPath = await saveToTempFile(clipboardContent.value)
        tempFilePath.value = tempPath

        const item: ClipboardContentItem = {
            type: 'clipboard',
            path: tempPath,
            name: `${t('clipboardImporter.content')}_${Date.now()}.txt`,
            size: new Blob([clipboardContent.value]).size,
            mimeType: 'text/plain',
            createdAt: Date.now(),
            content: clipboardContent.value,
            tempPath,
        }

        emit('select', item)
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)
        errorMessage.value = t('clipboardImporter.saveFailed', {
            error: errorMsg,
        })
        console.error('保存剪贴板内容失败:', error)
    } finally {
        loading.value = false
    }
}
</script>

<style scoped>
.clipboard-preview {
    max-height: 200px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-all;
}
</style>
