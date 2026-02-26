<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSettingsStore } from '@/stores/settings'
import { setI18nLanguage } from '@/i18n'
import type { LanguageMode } from '@/types/settings'

const { t } = useI18n()
const settingsStore = useSettingsStore()

const languageOptions = computed(() => [
    { title: t('settings.language.zhCN'), value: 'zh-CN' as LanguageMode },
    { title: t('settings.language.enUS'), value: 'en-US' as LanguageMode },
    { title: t('settings.language.system'), value: 'system' as LanguageMode },
])

const currentLanguage = computed({
    get: () => settingsStore.language,
    set: async (value: LanguageMode) => {
        await settingsStore.setLanguage(value)
        // 应用语言变化
        setI18nLanguage(settingsStore.actualLanguage)
    },
})
</script>

<template>
    <div class="d-flex align-center justify-space-between">
        <div>
            <div class="text-subtitle-1">
                {{ t('settings.language.label') }}
            </div>
        </div>
        <v-select
            v-model="currentLanguage"
            :items="languageOptions"
            density="compact"
            variant="outlined"
            hide-details
            style="max-width: 200px"
        />
    </div>
</template>
