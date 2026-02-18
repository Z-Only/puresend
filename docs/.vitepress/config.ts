import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'PureSend',
  description: '跨平台文件传输应用',
  base: '/puresend/',

  head: [['link', { rel: 'icon', href: '/puresend/favicon.ico' }]],

  themeConfig: {
    logo: '/logo.svg',

    nav: [
      { text: '首页', link: '/' },
      { text: '快速开始', link: '/getting-started' },
      {
        text: '功能',
        items: [{ text: '文件传输', link: '/features/file-transfer' }]
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
          items: [{ text: '文件传输', link: '/features/file-transfer' }]
        },
        {
          text: '开发指南',
          collapsed: false,
          items: [{ text: '环境搭建', link: '/development/setup' }]
        }
      ]
    },

    socialLinks: [{ icon: 'github', link: 'https://github.com/Z-Only/puresend' }],

    footer: {
      message: '基于 MIT 许可发布',
      copyright: 'Copyright © 2024-present PureSend'
    },

    search: {
      provider: 'local'
    },

    outline: {
      level: [2, 3]
    }
  },

  lang: 'zh-CN'
})
