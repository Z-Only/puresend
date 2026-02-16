<!-- 文件选择组件 - 支持拖拽和点击选择 -->
<template>
  <v-card
    class="file-selector"
    :class="{ 'file-selector--dragover': isDragover }"
    @dragover.prevent="handleDragOver"
    @dragleave.prevent="handleDragLeave"
    @drop.prevent="handleDrop"
    @click="openFileDialog"
  >
    <v-card-text class="d-flex flex-column align-center justify-center pa-8">
      <v-icon
        :icon="isDragover ? 'mdi-cloud-upload' : 'mdi-file-plus'"
        size="64"
        :color="isDragover ? 'primary' : 'grey'"
        class="mb-4"
      />
      
      <div class="text-h6 mb-2">
        {{ isDragover ? '松开以上传' : '拖拽文件到此处' }}
      </div>
      
      <div class="text-body-2 text-grey mb-4">
        或点击选择文件
      </div>
      
      <v-btn
        color="primary"
        variant="outlined"
        @click.stop="openFileDialog"
      >
        <v-icon icon="mdi-folder-open" class="mr-2" />
        选择文件
      </v-btn>
      
      <!-- 已选择的文件 -->
      <div v-if="selectedFile" class="mt-6 w-100">
        <v-divider class="mb-4" />
        <div class="d-flex align-center">
          <v-icon :icon="getFileIcon(selectedFile.type)" size="40" class="mr-3" />
          <div class="flex-grow-1 overflow-hidden">
            <div class="text-subtitle-1 text-truncate">{{ selectedFile.name }}</div>
            <div class="text-body-2 text-grey">{{ formatSize(selectedFile.size) }}</div>
          </div>
          <v-btn
            icon="mdi-close"
            variant="text"
            size="small"
            @click.stop="clearFile"
          />
        </div>
      </div>
    </v-card-text>
  </v-card>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { formatFileSize, getFileIcon as getFileIconName } from '../../types'

const emit = defineEmits<{
  (e: 'select', file: { path: string; name: string; size: number; type: string }): void
  (e: 'clear'): void
}>()

const isDragover = ref(false)
const selectedFile = ref<{ path: string; name: string; size: number; type: string } | null>(null)

function handleDragOver() {
  isDragover.value = true
}

function handleDragLeave() {
  isDragover.value = false
}

async function handleDrop(event: DragEvent) {
  isDragover.value = false
  
  const files = event.dataTransfer?.files
  if (files && files.length > 0) {
    const file = files[0]
    // 注意：在 Web 环境中无法获取完整路径，需要使用 Tauri 文件对话框
    // 这里仅作为 UI 展示，实际路径需要通过 Tauri API 获取
    selectedFile.value = {
      path: file.name, // Web 环境下只有文件名
      name: file.name,
      size: file.size,
      type: file.type || 'application/octet-stream'
    }
    emit('select', selectedFile.value)
  }
}

async function openFileDialog() {
  try {
    const selected = await open({
      multiple: false,
      title: '选择要传输的文件'
    })
    
    if (selected && typeof selected === 'string') {
      const name = selected.split(/[/\\]/).pop() || selected
      const extension = name.split('.').pop()?.toLowerCase() || ''
      
      selectedFile.value = {
        path: selected,
        name,
        size: 0, // 文件大小需要后端获取
        type: getMimeType(extension)
      }
      
      emit('select', selectedFile.value)
    }
  } catch (error) {
    console.error('打开文件对话框失败:', error)
  }
}

function clearFile() {
  selectedFile.value = null
  emit('clear')
}

function formatSize(bytes: number): string {
  return formatFileSize(bytes)
}

function getFileIcon(mimeType: string): string {
  const iconName = getFileIconName(mimeType)
  const iconMap: Record<string, string> = {
    image: 'mdi-file-image',
    video: 'mdi-file-video',
    audio: 'mdi-file-music',
    pdf: 'mdi-file-pdf-box',
    document: 'mdi-file-document',
    spreadsheet: 'mdi-file-excel',
    presentation: 'mdi-file-powerpoint',
    archive: 'mdi-folder-zip',
    text: 'mdi-file-document-outline',
    file: 'mdi-file'
  }
  return iconMap[iconName] || iconMap.file
}

function getMimeType(extension: string): string {
  const mimeTypes: Record<string, string> = {
    txt: 'text/plain',
    md: 'text/markdown',
    json: 'application/json',
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    png: 'image/png',
    gif: 'image/gif',
    webp: 'image/webp',
    pdf: 'application/pdf',
    mp4: 'video/mp4',
    mp3: 'audio/mpeg',
    zip: 'application/zip'
  }
  return mimeTypes[extension] || 'application/octet-stream'
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
