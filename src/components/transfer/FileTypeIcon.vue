<!-- 文件类型图标组件 -->
<template>
  <v-avatar :size="size" :color="iconColor" rounded>
    <v-icon :icon="iconName" :size="iconSize" color="white" />
  </v-avatar>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  mimeType: string
  size?: number
}>(), {
  size: 48
})

const iconSize = computed(() => Math.round(props.size * 0.5))

const iconName = computed(() => {
  const type = getFileType(props.mimeType)
  switch (type) {
    case 'image':
      return 'mdi-image'
    case 'video':
      return 'mdi-video'
    case 'audio':
      return 'mdi-music'
    case 'pdf':
      return 'mdi-file-pdf-box'
    case 'document':
      return 'mdi-file-document'
    case 'spreadsheet':
      return 'mdi-file-excel'
    case 'presentation':
      return 'mdi-file-powerpoint'
    case 'archive':
      return 'mdi-zip-box'
    case 'text':
      return 'mdi-file-document-outline'
    case 'code':
      return 'mdi-code-braces'
    default:
      return 'mdi-file'
  }
})

const iconColor = computed(() => {
  const type = getFileType(props.mimeType)
  switch (type) {
    case 'image':
      return 'pink'
    case 'video':
      return 'purple'
    case 'audio':
      return 'indigo'
    case 'pdf':
      return 'red'
    case 'document':
      return 'blue'
    case 'spreadsheet':
      return 'green'
    case 'presentation':
      return 'orange'
    case 'archive':
      return 'brown'
    case 'text':
      return 'grey'
    case 'code':
      return 'teal'
    default:
      return 'grey'
  }
})

function getFileType(mimeType: string): string {
  if (mimeType.startsWith('image/')) return 'image'
  if (mimeType.startsWith('video/')) return 'video'
  if (mimeType.startsWith('audio/')) return 'audio'
  if (mimeType === 'application/pdf') return 'pdf'
  if (mimeType.includes('word') || mimeType.includes('document')) return 'document'
  if (mimeType.includes('excel') || mimeType.includes('spreadsheet')) return 'spreadsheet'
  if (mimeType.includes('powerpoint') || mimeType.includes('presentation')) return 'presentation'
  if (mimeType.includes('zip') || mimeType.includes('compressed') || 
      mimeType.includes('rar') || mimeType.includes('7z') || mimeType.includes('tar')) return 'archive'
  if (mimeType.startsWith('text/')) return 'text'
  if (mimeType.includes('javascript') || mimeType.includes('typescript') || 
      mimeType.includes('json') || mimeType.includes('xml')) return 'code'
  return 'file'
}
</script>
