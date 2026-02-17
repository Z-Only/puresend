<template>
    <div class="content-type-selector">
        <v-btn-toggle
            v-model="selectedType"
            mandatory
            variant="outlined"
            rounded="0"
            class="d-flex flex-wrap"
        >
            <v-btn
                v-for="type in contentTypes"
                :key="type"
                :value="type"
                :title="t(getContentTypeInfo(type).descriptionKey)"
                class="flex-grow-1 content-type-btn"
            >
                <v-icon :icon="getContentTypeInfo(type).icon" />
                <span class="btn-text">{{
                    t(getContentTypeInfo(type).labelKey)
                }}</span>
            </v-btn>
        </v-btn-toggle>
    </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ContentType } from '../../types'
import { getContentTypeInfo } from '../../types'

const { t } = useI18n()

const emit = defineEmits<{
    (e: 'change', type: ContentType): void
}>()

const contentTypes: ContentType[] = [
    'file',
    'folder',
    'clipboard',
    'text',
    'media',
    'app',
]
const selectedType = ref<ContentType>('file')

watch(selectedType, (newType) => {
    emit('change', newType)
})
</script>

<style scoped>
.content-type-btn {
    display: grid !important;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    justify-items: center;
}

.content-type-btn .v-icon {
    grid-column: 1;
}

.content-type-btn .btn-text {
    grid-column: 2;
    text-align: center;
}
</style>
