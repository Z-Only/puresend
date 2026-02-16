<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import SettingsGroup from '@/components/settings/SettingsGroup.vue'
import ThemeSelector from '@/components/settings/ThemeSelector.vue'
import LanguageSelector from '@/components/settings/LanguageSelector.vue'

const { t } = useI18n()
</script>

<template>
    <v-container class="settings-view" max-width="800">
        <h1 class="text-h4 mb-6">
            {{ t('settings.title') }}
        </h1>

        <SettingsGroup :title="t('settings.appearance')">
            <div class="d-flex flex-column ga-4">
                <ThemeSelector />
                <v-divider />
                <LanguageSelector />
            </div>
        </SettingsGroup>
    </v-container>
</template>

<style scoped>
.settings-view {
    padding: 24px;
}
</style>
