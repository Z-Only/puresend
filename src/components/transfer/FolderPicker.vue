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
            </v-card-text>
        </v-card>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import type { ContentItem } from '../../types'
import { mdiFolder, mdiFolderOpen } from '@mdi/js'
import { usePlatform } from '@/composables'
// 移动端文件系统 API
import { AndroidFs } from 'tauri-plugin-android-fs-api'

const { t } = useI18n()
const { isMobile } = usePlatform()

/** 文件信息接口 */
interface FileInfo {
    path: string
    name: string
    size: number
    relative_path: string
}

/** 带文件列表的内容项 */
interface FolderContentItem extends ContentItem {
    files?: FileInfo[]
}

const emit = defineEmits<{
    (e: 'select', item: FolderContentItem): void
}>()

const loading = ref(false)
const selectedFolder = ref<FolderContentItem | null>(null)
const errorMessage = ref('')

/**
 * 递归获取文件夹下的所有文件（桌面端）
 */
async function getFilesInFolder(folderPath: string): Promise<FileInfo[]> {
    try {
        const files = await invoke<FileInfo[]>('get_files_in_folder', {
            folderPath,
        })
        return files || []
    } catch (error) {
        console.warn('获取文件夹文件列表失败:', error)
        return []
    }
}

/**
 * 递归读取 Android 文件夹（移动端）
 */
async function readDirRecursive(
    dirUri: string,
    basePath: string = ''
): Promise<FileInfo[]> {
    const entries = await AndroidFs.readDir(dirUri)
    const files: FileInfo[] = []
    for (const entry of entries) {
        const name = await AndroidFs.getName(entry.uri)
        if (entry.isDir) {
            const subFiles = await readDirRecursive(
                entry.uri,
                basePath ? `${basePath}/${name}` : name
            )
            files.push(...subFiles)
        } else {
            const size = await AndroidFs.getByteLength(entry.uri)
            files.push({
                path: entry.uri,
                name,
                size,
                relative_path: basePath ? `${basePath}/${name}` : name,
            })
        }
    }
    return files
}

async function pickFolder() {
    loading.value = true
    errorMessage.value = ''

    try {
        if (isMobile.value) {
            // 移动端：使用 Android 文件系统插件
            const dirUri = await AndroidFs.showOpenDirPicker()

            if (!dirUri) {
                return
            }

            const name = await AndroidFs.getName(dirUri)
            const files = await readDirRecursive(dirUri)

            const totalSize = files.reduce((sum, f) => sum + f.size, 0)

            selectedFolder.value = {
                type: 'folder',
                path: dirUri,
                name,
                size: totalSize,
                mimeType: 'application/x-directory',
                createdAt: Date.now(),
                files: files,
            }

            if (files.length === 0) {
                errorMessage.value = t('folderPicker.emptyFolder')
            }

            emit('select', selectedFolder.value)
        } else {
            // 桌面端：使用 Tauri 原生对话框
            const selected = await open({
                multiple: false,
                directory: true,
                title: t('folderPicker.selectFolder'),
            })

            if (selected && typeof selected === 'string') {
                const name = selected.split(/[/\\]/).pop() || selected

                const files = await getFilesInFolder(selected)

                const totalSize = files.reduce((sum, f) => sum + f.size, 0)

                const normalizedPath = selected.replace(/\\/g, '/')

                selectedFolder.value = {
                    type: 'folder',
                    path: normalizedPath,
                    name,
                    size: totalSize,
                    mimeType: 'application/x-directory',
                    createdAt: Date.now(),
                    files: files.map((f) => ({
                        ...f,
                        path: f.path?.replace(/\\/g, '/') || f.path,
                        relative_path:
                            f.relative_path?.replace(/\\/g, '/') || '',
                    })),
                }

                if (files.length === 0) {
                    errorMessage.value = t('folderPicker.emptyFolder')
                }

                emit('select', selectedFolder.value)
            }
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
