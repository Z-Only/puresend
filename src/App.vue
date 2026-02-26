<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useSettingsStore } from '@/stores/settings'
import { useDiscoveryStore } from '@/stores/discovery'
import { setI18nLanguage, type AppLocale } from '@/i18n'
import { useTheme } from 'vuetify'
import {
    mdiSend,
    mdiWifiPlus,
    mdiHistory,
    mdiCog,
    mdiChevronLeft,
    mdiChevronRight,
} from '@mdi/js'
import { usePlatform } from '@/composables'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useFontSize } from '@/composables/useFontSize'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const settingsStore = useSettingsStore()
const vuetifyTheme = useTheme()
const discoveryStore = useDiscoveryStore()

const { isMobile } = usePlatform()

// 初始化字体大小
useFontSize()

// 垂直 Tab 栏折叠状态
const isNavRailMode = ref(false)

// 计算当前是否为水平布局（顶部或底部）
const isHorizontalLayout = computed(() => {
    if (isMobile.value) return true
    return (
        settingsStore.tabLayout === 'horizontal-top' ||
        settingsStore.tabLayout === 'horizontal-bottom'
    )
})

// 计算当前是否为水平底部布局
const isHorizontalBottom = computed(() => {
    return settingsStore.tabLayout === 'horizontal-bottom'
})

// 计算当前是否为垂直布局
const isVerticalLayout = computed(() => {
    if (isMobile.value) return false
    return (
        settingsStore.tabLayout === 'vertical-left' ||
        settingsStore.tabLayout === 'vertical-right'
    )
})

// 计算垂直导航栏位置
const navDrawerLocation = computed(() => {
    return settingsStore.tabLayout === 'vertical-right' ? 'right' : 'left'
})

const currentRoute = computed(() => route.name as string)

const navigationItems = computed(() => [
    {
        title: t('nav.send'),
        icon: mdiSend,
        route: 'Send',
    },
    {
        title: t('nav.receive'),
        icon: mdiWifiPlus,
        route: 'Receive',
    },
    { title: t('nav.history'), icon: mdiHistory, route: 'History' },
    { title: t('nav.settings'), icon: mdiCog, route: 'Settings' },
])

function navigateTo(routeName: string) {
    router.push({ name: routeName })
}

// 系统主题变化回调
let cleanupThemeWatcher: (() => void) | null = null
let unlistenMenuEvent: UnlistenFn | null = null

function handleSystemThemeChange(theme: 'light' | 'dark') {
    vuetifyTheme.change(theme)
}

// 监听语言变化，更新 macOS 菜单栏
watch(
    () => settingsStore.actualLanguage,
    async (newLang) => {
        try {
            await invoke('update_menu_language', { lang: newLang })
        } catch {
            // 非 macOS 平台或 Tauri 不可用时忽略
        }
    }
)

onMounted(async () => {
    // 应用保存的设置
    await settingsStore.loadSettings()

    // 设置语言
    setI18nLanguage(settingsStore.actualLanguage as AppLocale)

    // 设置主题
    vuetifyTheme.change(settingsStore.actualTheme)

    // 初始化设备发现服务，使用设置中的设备名称
    const deviceName = await settingsStore.getDeviceName()
    await discoveryStore.initialize(deviceName)

    // 监听系统主题变化
    cleanupThemeWatcher = settingsStore.watchSystemTheme(
        handleSystemThemeChange
    )

    // 初始化 macOS 菜单栏语言
    try {
        await invoke('update_menu_language', {
            lang: settingsStore.actualLanguage,
        })
    } catch {
        // 非 macOS 平台或 Tauri 不可用时忽略
    }

    // 监听菜单事件
    try {
        unlistenMenuEvent = await listen<string>('menu-event', (event) => {
            switch (event.payload) {
                case 'about':
                    navigateTo('Settings')
                    break
                case 'send_file':
                    navigateTo('Send')
                    break
            }
        })
    } catch {
        // Tauri 不可用时忽略
    }
})

onUnmounted(() => {
    if (cleanupThemeWatcher) {
        cleanupThemeWatcher()
    }
    if (unlistenMenuEvent) {
        unlistenMenuEvent()
    }
})
</script>

<template>
    <v-app>
        <!-- 桌面端水平布局：顶部应用栏 -->
        <v-app-bar
            v-if="isHorizontalLayout && !isMobile && !isHorizontalBottom"
            density="comfortable"
        >
            <v-tabs
                :model-value="currentRoute"
                align-tabs="center"
                grow
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

        <!-- 水平底部布局的应用栏 -->
        <v-app-bar
            v-if="isHorizontalLayout && !isMobile && isHorizontalBottom"
            density="comfortable"
            location="bottom"
        >
            <v-tabs
                :model-value="currentRoute"
                align-tabs="center"
                grow
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

        <!-- 垂直布局：侧边导航栏 -->
        <v-navigation-drawer
            v-if="isVerticalLayout"
            permanent
            :rail="isNavRailMode"
            :location="navDrawerLocation"
            :width="150"
        >
            <v-list
                class="d-flex flex-column justify-center nav-list"
                style="height: calc(100% - 48px)"
                nav
            >
                <v-list-item
                    v-for="item in navigationItems"
                    :key="item.route"
                    :value="item.route"
                    :active="currentRoute === item.route"
                    color="primary"
                    class="justify-center nav-item"
                    @click="navigateTo(item.route)"
                >
                    <div class="d-flex align-center justify-center">
                        <v-icon :icon="item.icon" />
                        <span v-if="!isNavRailMode" class="ml-3">{{
                            item.title
                        }}</span>
                    </div>
                    <v-tooltip
                        v-if="isNavRailMode"
                        activator="parent"
                        :location="
                            navDrawerLocation === 'left' ? 'end' : 'start'
                        "
                    >
                        {{ item.title }}
                    </v-tooltip>
                </v-list-item>
            </v-list>

            <template #append>
                <div class="d-flex justify-center pa-2">
                    <v-btn
                        variant="text"
                        size="small"
                        :icon="true"
                        @click="isNavRailMode = !isNavRailMode"
                    >
                        <v-icon
                            :icon="
                                isNavRailMode
                                    ? navDrawerLocation === 'left'
                                        ? mdiChevronRight
                                        : mdiChevronLeft
                                    : navDrawerLocation === 'left'
                                      ? mdiChevronLeft
                                      : mdiChevronRight
                            "
                        />
                        <v-tooltip
                            activator="parent"
                            :location="
                                navDrawerLocation === 'left' ? 'end' : 'start'
                            "
                        >
                            {{
                                isNavRailMode
                                    ? t('settings.about.expand')
                                    : t('settings.about.collapse')
                            }}
                        </v-tooltip>
                    </v-btn>
                </div>
            </template>
        </v-navigation-drawer>

        <!-- 主内容区域 -->
        <v-main :class="{ 'mobile-main': isMobile }">
            <div
                class="main-content-scroll"
                :class="{ 'mobile-content-scroll': isMobile }"
            >
                <router-view v-slot="{ Component }">
                    <transition name="fade" mode="out-in">
                        <component :is="Component" />
                    </transition>
                </router-view>
            </div>
        </v-main>

        <!-- 移动端底部导航栏 -->
        <v-bottom-navigation
            v-if="isMobile"
            :model-value="currentRoute"
            grow
            class="mobile-bottom-nav"
            @update:model-value="navigateTo"
        >
            <v-btn
                v-for="item in navigationItems"
                :key="item.route"
                :value="item.route"
            >
                <v-icon :icon="item.icon" />
                <span>{{ item.title }}</span>
            </v-btn>
        </v-bottom-navigation>
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

/* 禁止全局滚动 */
html,
body,
#app {
    overflow: hidden;
    height: 100%;
    margin: 0;
    /* 隐藏 macOS 原生滚动条 */
    scrollbar-width: none; /* Firefox */
    -ms-overflow-style: none; /* IE/Edge */
}

/* 内容区域滚动 */
.main-content-scroll {
    height: calc(
        100vh - var(--v-layout-top, 0px) - var(--v-layout-bottom, 0px)
    );
    overflow-y: auto;
    overflow-x: hidden;
}

/* 自定义滚动条样式 - Webkit */
.main-content-scroll::-webkit-scrollbar {
    width: 6px;
}

.main-content-scroll::-webkit-scrollbar-track {
    background: transparent;
}

.main-content-scroll::-webkit-scrollbar-thumb {
    background-color: rgba(128, 128, 128, 0.3);
    border-radius: 3px;
}

.main-content-scroll::-webkit-scrollbar-thumb:hover {
    background-color: rgba(128, 128, 128, 0.5);
}

/* 自定义滚动条样式 - Firefox */
.main-content-scroll {
    scrollbar-width: thin;
    scrollbar-color: rgba(128, 128, 128, 0.3) transparent;
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

/* 导航栏列表项固定高度和字体，防止展开/收起时变化 */
.nav-list .nav-item {
    min-height: 48px !important;
    height: 48px !important;
    font-size: 14px !important;
}

.nav-list .nav-item .v-icon {
    font-size: 24px !important;
}

/* 移动端沉浸式状态栏适配 */
.mobile-main {
    padding-top: env(safe-area-inset-top, 0px) !important;
}

@supports not (padding-top: env(safe-area-inset-top)) {
    .mobile-main {
        padding-top: 24px !important;
    }
}

/* 移动端底部导航栏 */
.mobile-bottom-nav {
    position: fixed !important;
    bottom: 0 !important;
    left: 0 !important;
    right: 0 !important;
    z-index: 1000 !important;
}

/* 移动端内容区域底部留出导航栏空间 */
.mobile-content-scroll {
    padding-bottom: 56px;
}
</style>
