<!-- 文件夹选择器组件 -->
<template>
    <div class="folder-picker">
        <v-card variant="outlined" class="pa-4">
            <v-card-text>
                <div class="d-flex align-center mb-3">
                    <v-icon :icon="mdiFolder" class="mr-2" color="primary" />
                    <span class="text-subtitle-1 font-weight-bold">
                        {{ t('folderPicker.title') }}
                    </span>
                </div>

                <div class="text-body-2 text-grey mb-3">
                    {{ t('folderPicker.description') }}
                </div>

                <v-btn
                    color="primary"
                    :loading="loading"
                    block
                    class="text-center"
                    @click="pickFolder"
                >
                    <v-icon :icon="mdiFolderOpen" class="mr-2" />
                    {{ t('folderPicker.selectFolder') }}
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
import { useI18n } from 'vue-i18n'
import { open } from '@tauri-apps/plugin-dialog'
import { getFileMetadata } from '../../services/transferService'
import type { ContentItem } from '../../types'
import { mdiFolder, mdiFolderOpen } from '@mdi/js'

const { t } = useI18n()

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
            title: t('folderPicker.selectFolder'),
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
            errorMessage.value = t('folderPicker.noFolderSelected')
        }
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)
        errorMessage.value = t('folderPicker.selectFailed', { error: errorMsg })
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
