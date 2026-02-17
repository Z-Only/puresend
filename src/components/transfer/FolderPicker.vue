<!-- 文件夹选择器组件 -->
<template>
    <div class="folder-picker">
        <v-card variant="outlined" class="pa-4">
            <v-card-text>
                <div class="d-flex align-center mb-3">
                    <v-icon :icon="mdiFolder" class="mr-2" color="primary" />
                    <span class="text-subtitle-1 font-weight-bold"
                        >选择文件夹</span
                    >
                </div>

                <div class="text-body-2 text-grey mb-3">
                    选择整个文件夹进行传输
                </div>

                <v-btn
                    color="primary"
                    :loading="loading"
                    block
                    class="text-center"
                    @click="pickFolder"
                >
                    <v-icon :icon="mdiFolderOpen" class="mr-2" />
                    选择文件夹
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

                <!-- 已选择的文件夹信息 -->
                <div v-if="selectedFolder" class="mt-4">
                    <v-divider class="mb-3" />
                    <div class="d-flex align-center">
                        <v-icon
                            :icon="mdiFolder"
                            size="40"
                            color="primary"
                            class="mr-3"
                        />
                        <div class="flex-grow-1">
                            <div class="text-subtitle-1">
                                {{ selectedFolder.name }}
                            </div>
                            <div class="text-body-2 text-grey">
                                {{ selectedFolder.path }}
                            </div>
                        </div>
                    </div>
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
import { mdiFolder, mdiFolderOpen } from '@mdi/js'

const emit = defineEmits<{
    (e: 'select', item: ContentItem): void
}>()

const loading = ref(false)
const selectedFolder = ref<ContentItem | null>(null)
const errorMessage = ref('')

async function pickFolder() {
    loading.value = true
    errorMessage.value = ''
    try {
        const selected = await open({
            multiple: false,
            directory: true,
            title: '选择文件夹',
        })

        if (selected && typeof selected === 'string') {
            const name = selected.split(/[/\\]/).pop() || selected

            // 获取文件夹元数据
            let size = 0
            try {
                const metadata = await getFileMetadata(selected)
                size = metadata.size
            } catch (metaError) {
                console.warn('获取文件夹元数据失败:', metaError)
            }

            selectedFolder.value = {
                type: 'folder',
                path: selected,
                name,
                size,
                mimeType: 'application/x-directory',
                createdAt: Date.now(),
            }

            emit('select', selectedFolder.value)
        } else {
            errorMessage.value = '未选择任何文件夹'
        }
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)
        errorMessage.value = `选择文件夹失败：${errorMsg}`
        console.error('选择文件夹失败:', error)
    } finally {
        loading.value = false
    }
}
</script>

<style scoped>
.folder-picker {
    width: 100%;
}
</style>
