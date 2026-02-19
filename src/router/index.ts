import { createRouter, createWebHistory } from 'vue-router'

const routes = [
    {
        path: '/',
        name: 'Send',
        component: () => import('@/views/SendView.vue'),
        meta: { titleKey: 'nav.send' },
    },
    {
        path: '/receive',
        name: 'Receive',
        component: () => import('@/views/ReceiveView.vue'),
        meta: { titleKey: 'nav.receive' },
    },
    {
        path: '/history',
        name: 'History',
        component: () => import('@/views/HistoryView.vue'),
        meta: { titleKey: 'nav.history' },
    },
    {
        path: '/settings',
        name: 'Settings',
        component: () => import('@/views/SettingsView.vue'),
        meta: { titleKey: 'nav.settings' },
    },
    {
        path: '/share-link',
        name: 'ShareLink',
        component: () => import('@/views/ShareLinkView.vue'),
        meta: { titleKey: 'nav.shareLink' },
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
