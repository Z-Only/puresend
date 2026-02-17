<!-- 剪贴板导入组件 -->
<template>
    <div class="clipboard-importer">
        <v-card variant="outlined" class="pa-4">
            <v-card-text>
                <div class="d-flex align-center mb-3">
                    <v-icon icon="mdi-clipboard" class="mr-2" color="primary" />
                    <span class="text-subtitle-1 font-weight-bold"
                        >从剪贴板导入</span
                    >
                </div>

                <div v-if="clipboardContent" class="mb-3">
                    <div class="text-body-2 text-grey mb-2">预览：</div>
                    <v-card variant="tonal" class="pa-3">
                        <div class="text-body-2 clipboard-preview">
                            {{ clipboardContent }}
                        </div>
                    </v-card>
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
                        {{ clipboardContent ? '重新读取' : '读取剪贴板' }}
                    </v-btn>

                    <v-btn
                        v-if="clipboardContent"
                        color="success"
                        min-width="120"
                        class="text-center"
                        @click="confirmImport"
                    >
                        确认导入
                    </v-btn>
                </div>
            </v-card-text>
        </v-card>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { readText } from '@tauri-apps/plugin-clipboard-manager'
import type { ContentItem } from '../../types'

const emit = defineEmits<{
    (e: 'select', item: ContentItem): void
}>()

const loading = ref(false)
const clipboardContent = ref('')
const errorMessage = ref('')

async function importFromClipboard() {
    loading.value = true
    errorMessage.value = ''
    try {
        const text = await readText()
        if (text) {
            clipboardContent.value = text
        } else {
            clipboardContent.value = ''
            errorMessage.value = '剪贴板为空，请先复制一些文本内容'
        }
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)
        errorMessage.value = `读取剪贴板失败：${errorMsg}`
        console.error('读取剪贴板失败:', error)
    } finally {
        loading.value = false
    }
}

function confirmImport() {
    if (!clipboardContent.value) return

    const item: ContentItem = {
        type: 'clipboard',
        path: 'clipboard://current',
        name: '剪贴板内容',
        size: clipboardContent.value.length,
        mimeType: 'text/plain',
        createdAt: Date.now(),
    }

    emit('select', item)
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
