import { createRouter, createWebHistory } from 'vue-router'

const routes = [
    {
        path: '/',
        name: 'Transfer',
        component: () => import('@/views/TransferView.vue'),
        meta: { title: '文件传输' },
    },
    {
        path: '/history',
        name: 'History',
        component: () => import('@/views/HistoryView.vue'),
        meta: { title: '传输历史' },
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
