import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'
import vuetify from 'vite-plugin-vuetify'
import eslint from 'vite-plugin-eslint'
import AutoImport from 'unplugin-auto-import/vite'
import Components from 'unplugin-vue-components/vite'

const host = process.env.TAURI_DEV_HOST

// https://vite.dev/config/
export default defineConfig(async () => ({
    plugins: [
        vue(),
        vuetify({ autoImport: true }),
        eslint(),
        AutoImport({
            imports: ['vue', 'vue-router'],
            dts: false,
            eslintrc: {
                enabled: false,
                filepath: './.eslintrc-auto-import.json',
            },
        }),
        Components({ dts: true }),
    ],
    resolve: {
        alias: {
            '@': resolve(__dirname, 'src'),
        },
    },
    define: {
        __APP_VERSION__: JSON.stringify(
            process.env.npm_package_version || '0.1.0'
        ),
    },

    // 构建优化配置
    build: {
        // 设置 chunk 大小警告限制为 600 kB
        chunkSizeWarningLimit: 600,
        rollupOptions: {
            output: {
                // 使用 Rolldown 的 advancedChunks 替代已过时的 manualChunks
                // 类型断言用于绕过标准 Vite 类型定义的限制
                advancedChunks: {
                    groups: [
                        // Vue 相关库
                        {
                            name: 'vendor-vue',
                            test: /\/node_modules\/(vue|vue-router|pinia)\//,
                        },
                        // Vuetify UI 框架
                        {
                            name: 'vendor-vuetify',
                            test: /\/node_modules\/vuetify\//,
                        },
                        // 国际化
                        {
                            name: 'vendor-i18n',
                            test: /\/node_modules\/vue-i18n\//,
                        },
                        // 其他第三方库
                        { name: 'vendor', test: /\/node_modules\/qrcode\// },
                    ],
                },
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
            } as any,
        },
    },

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
        port: 1420,
        strictPort: true,
        host: host || false,
        hmr: host
            ? {
                  protocol: 'ws',
                  host,
                  port: 1421,
              }
            : undefined,
        watch: {
            // 3. tell Vite to ignore watching `src-tauri`
            ignored: ['**/src-tauri/**'],
        },
    },
}))
