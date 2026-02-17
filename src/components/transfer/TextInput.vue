<!-- 文本输入组件 -->
<template>
    <div class="text-input">
        <v-card variant="outlined" class="pa-4">
            <v-card-text>
                <div class="d-flex align-center mb-3">
                    <v-icon :icon="mdiTextBox" class="mr-2" color="primary" />
                    <span class="text-subtitle-1 font-weight-bold">
                        {{ t('textInput.title') }}
                    </span>
                </div>

                <v-textarea
                    v-model="textContent"
                    :label="t('textInput.placeholder')"
                    rows="6"
                    variant="outlined"
                    class="mb-3"
                    @blur="handleTextComplete"
                />

                <div v-if="textContent" class="text-body-2 text-grey mb-3">
                    {{ t('textInput.charCount') }}：{{ textContent.length }}
                </div>

                <v-btn
                    color="primary"
                    :disabled="!textContent"
                    block
                    class="text-center"
                    @click="confirmText"
                >
                    {{ t('textInput.addToSend') }}
                </v-btn>
            </v-card-text>
        </v-card>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ContentItem } from '../../types'
import { mdiTextBox } from '@mdi/js'

const { t } = useI18n()

const emit = defineEmits<{
    (e: 'select', item: ContentItem): void
}>()

const textContent = ref('')

function handleTextComplete() {
    // 可以在这里添加自动保存等逻辑
}

function confirmText() {
    if (!textContent.value) return

    const item: ContentItem = {
        type: 'text',
        path: 'text://input',
        name: t('textInput.content'),
        size: textContent.value.length,
        mimeType: 'text/plain',
        createdAt: Date.now(),
        metadata: {
            content: textContent.value,
        },
    }

    emit('select', item)
}
</script>

<style scoped>
.text-input {
    width: 100%;
}
</style>
