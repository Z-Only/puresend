import { createRouter, createWebHistory } from 'vue-router'

const routes = [
    {
        path: '/',
        name: 'Send',
        component: () => import('@/views/SendView.vue'),
        meta: { title: '发送文件' },
    },
    {
        path: '/receive',
        name: 'Receive',
        component: () => import('@/views/ReceiveView.vue'),
        meta: { title: '接收文件' },
    },
    {
        path: '/history',
        name: 'History',
        component: () => import('@/views/HistoryView.vue'),
        meta: { title: '传输历史' },
    },
    {
        path: '/settings',
        name: 'Settings',
        component: () => import('@/views/SettingsView.vue'),
        meta: { title: '设置' },
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
