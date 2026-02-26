<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTheme } from 'vuetify'
import { useSettingsStore } from '@/stores/settings'
import type { ThemeMode } from '@/types/settings'

const { t } = useI18n()
const settingsStore = useSettingsStore()
const vuetifyTheme = useTheme()

const themeOptions = computed(() => [
    { title: t('settings.theme.light'), value: 'light' as ThemeMode },
    { title: t('settings.theme.dark'), value: 'dark' as ThemeMode },
    { title: t('settings.theme.system'), value: 'system' as ThemeMode },
])

const currentTheme = computed({
    get: () => settingsStore.theme,
    set: async (value: ThemeMode) => {
        await settingsStore.setTheme(value)
        // 应用主题变化到 Vuetify
        vuetifyTheme.change(settingsStore.actualTheme)
    },
})
</script>

<template>
    <div class="d-flex align-center justify-space-between">
        <div>
            <div class="text-subtitle-1">
                {{ t('settings.theme.label') }}
            </div>
        </div>
        <v-select
            v-model="currentTheme"
            :items="themeOptions"
            density="compact"
            variant="outlined"
            hide-details
            style="max-width: 200px"
        />
    </div>
</template>
