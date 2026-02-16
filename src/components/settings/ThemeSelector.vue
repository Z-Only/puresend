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
        vuetifyTheme.global.name.value = settingsStore.actualTheme
    },
})
</script>

<template>
    <v-radio-group
        v-model="currentTheme"
        :label="t('settings.theme.label')"
        inline
        hide-details
    >
        <v-radio
            v-for="option in themeOptions"
            :key="option.value"
            :label="option.title"
            :value="option.value"
        />
    </v-radio-group>
</template>
