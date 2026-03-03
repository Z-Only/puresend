import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'PureSend',
  description: '跨平台文件传输应用',
  base: '/puresend/',

  head: [
    // Favicon - 多格式多尺寸适配
    ['link', { rel: 'icon', type: 'image/x-icon', href: '/puresend/favicon.ico' }],
    ['link', { rel: 'icon', type: 'image/png', sizes: '32x32', href: '/puresend/favicon-32x32.png' }],
    ['link', { rel: 'icon', type: 'image/png', sizes: '128x128', href: '/puresend/favicon-128x128.png' }],
    ['link', { rel: 'icon', type: 'image/png', sizes: '192x192', href: '/puresend/favicon-192x192.png' }],
    // Apple Touch Icon
    ['link', { rel: 'apple-touch-icon', href: '/puresend/logo.png' }],
  ],

  locales: {
    root: {
      label: '简体中文',
      lang: 'zh-CN',
      themeConfig: {
        logo: '/logo.png',

        nav: [
          { text: '首页', link: '/' },
          { text: '快速开始', link: '/getting-started' },
          {
            text: '功能',
            items: [
              { text: '文件传输', link: '/features/file-transfer' },
              { text: '云盘中转', link: '/features/cloud-storage' },
              { text: 'Web 下载', link: '/features/web-download' },
              { text: 'Web 上传', link: '/features/web-upload' },
              { text: '传输安全', link: '/features/security' },
              { text: '传输历史', link: '/features/history' },
              { text: '应用设置', link: '/features/settings' },
            ]
          },
          {
            text: '开发',
            items: [{ text: '环境搭建', link: '/development/setup' }]
          }
        ],

        sidebar: {
          '/': [
            {
              text: '开始',
              items: [
                { text: '首页', link: '/' },
                { text: '快速开始', link: '/getting-started' }
              ]
            },
            {
              text: '功能',
              collapsed: false,
              items: [
                { text: '文件传输', link: '/features/file-transfer' },
                { text: '云盘中转', link: '/features/cloud-storage' },
                { text: 'Web 下载', link: '/features/web-download' },
                { text: 'Web 上传', link: '/features/web-upload' },
                { text: '传输安全', link: '/features/security' },
                { text: '传输历史', link: '/features/history' },
                { text: '应用设置', link: '/features/settings' },
              ]
            },
            {
              text: '开发指南',
              collapsed: false,
              items: [{ text: '环境搭建', link: '/development/setup' }]
            }
          ]
        },

        footer: {
          message: '基于 MIT 许可发布',
          copyright: 'Copyright © 2024-present PureSend'
        }
      }
    },

    en: {
      label: 'English',
      lang: 'en-US',
      themeConfig: {
        logo: '/logo.png',

        nav: [
          { text: 'Home', link: '/en/' },
          { text: 'Getting Started', link: '/en/getting-started' },
          {
            text: 'Features',
            items: [
              { text: 'File Transfer', link: '/en/features/file-transfer' },
              { text: 'Cloud Storage', link: '/en/features/cloud-storage' },
              { text: 'Web Download', link: '/en/features/web-download' },
              { text: 'Web Upload', link: '/en/features/web-upload' },
              { text: 'Transfer Security', link: '/en/features/security' },
              { text: 'Transfer History', link: '/en/features/history' },
              { text: 'App Settings', link: '/en/features/settings' },
            ]
          },
          {
            text: 'Development',
            items: [{ text: 'Setup', link: '/en/development/setup' }]
          }
        ],

        sidebar: {
          '/en/': [
            {
              text: 'Getting Started',
              items: [
                { text: 'Home', link: '/en/' },
                { text: 'Getting Started', link: '/en/getting-started' }
              ]
            },
            {
              text: 'Features',
              collapsed: false,
              items: [
                { text: 'File Transfer', link: '/en/features/file-transfer' },
                { text: 'Cloud Storage', link: '/en/features/cloud-storage' },
                { text: 'Web Download', link: '/en/features/web-download' },
                { text: 'Web Upload', link: '/en/features/web-upload' },
                { text: 'Transfer Security', link: '/en/features/security' },
                { text: 'Transfer History', link: '/en/features/history' },
                { text: 'App Settings', link: '/en/features/settings' },
              ]
            },
            {
              text: 'Development',
              collapsed: false,
              items: [{ text: 'Setup', link: '/en/development/setup' }]
            }
          ]
        },

        footer: {
          message: 'Released under the MIT License',
          copyright: 'Copyright © 2024-present PureSend'
        }
      }
    }
  },

  themeConfig: {
    socialLinks: [{ icon: 'github', link: 'https://github.com/Z-Only/puresend' }],

    search: {
      provider: 'local'
    },

    outline: {
      level: [2, 3]
    }
  }
})
