import { createRouter, createWebHistory } from 'vue-router'
import App from '@/App.vue'

const routes = [
    {
        path: '/',
        name: 'Home',
        component: App,
    },
]

const router = createRouter({
    history: createWebHistory(),
    routes,
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    scrollBehavior(_to, _from, _savedPosition) {
        // 始终滚动到顶部
        return { top: 0 }
    },
})

export default router
