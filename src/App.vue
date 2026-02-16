<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'

const route = useRoute()
const router = useRouter()

const currentRoute = computed(() => route.name as string)

const navigationItems = [
    { title: '文件传输', icon: 'mdi-swap-horizontal', route: 'Transfer' },
    { title: '传输历史', icon: 'mdi-history', route: 'History' },
]

function navigateTo(routeName: string) {
    router.push({ name: routeName })
}
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
