<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useSettingsStore } from '@/stores/settings'
import { setI18nLanguage, type AppLocale } from '@/i18n'
import { useTheme } from 'vuetify'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const settingsStore = useSettingsStore()
const vuetifyTheme = useTheme()

const currentRoute = computed(() => route.name as string)

const navigationItems = computed(() => [
    {
        title: t('nav.send'),
        icon: 'mdi-send',
        route: 'Send',
    },
    {
        title: t('nav.receive'),
        icon: 'mdi-wifi-plus',
        route: 'Receive',
    },
    { title: t('nav.history'), icon: 'mdi-history', route: 'History' },
    { title: t('nav.settings'), icon: 'mdi-cog', route: 'Settings' },
])

function navigateTo(routeName: string) {
    router.push({ name: routeName })
}

// 系统主题变化回调
let cleanupThemeWatcher: (() => void) | null = null

function handleSystemThemeChange(theme: 'light' | 'dark') {
    vuetifyTheme.global.name.value = theme
}

onMounted(async () => {
    // 应用保存的设置
    await settingsStore.loadSettings()

    // 设置语言
    setI18nLanguage(settingsStore.actualLanguage as AppLocale)

    // 设置主题
    vuetifyTheme.global.name.value = settingsStore.actualTheme

    // 监听系统主题变化
    cleanupThemeWatcher = settingsStore.watchSystemTheme(
        handleSystemThemeChange
    )
})

onUnmounted(() => {
    if (cleanupThemeWatcher) {
        cleanupThemeWatcher()
    }
})
</script>

<template>
    <v-app>
        <!-- 顶部应用栏 -->
        <v-app-bar color="primary" density="comfortable">
            <v-app-bar-title>PureSend</v-app-bar-title>

            <v-spacer />

            <!-- 导航标签 -->
            <v-tabs
                v-model="currentRoute"
                color="white"
                align-tabs="center"
                @update:model-value="navigateTo"
            >
                <v-tab
                    v-for="item in navigationItems"
                    :key="item.route"
                    :value="item.route"
                >
                    <v-icon :icon="item.icon" class="mr-2" />
                    {{ item.title }}
                </v-tab>
            </v-tabs>
        </v-app-bar>

        <!-- 主内容区域 -->
        <v-main>
            <router-view v-slot="{ Component }">
                <transition name="fade" mode="out-in">
                    <component :is="Component" />
                </transition>
            </router-view>
        </v-main>
    </v-app>
</template>

<style>
/* 全局样式 */
:root {
    font-family: 'Roboto', Inter, Avenir, Helvetica, Arial, sans-serif;
    font-size: 16px;
    line-height: 1.5;
    font-weight: 400;
    font-synthesis: none;
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    -webkit-text-size-adjust: 100%;
}

/* 页面过渡动画 */
.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}
</style>
