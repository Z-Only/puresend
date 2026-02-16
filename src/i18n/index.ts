/**
 * 国际化配置
 */
import { createI18n } from 'vue-i18n'
import zhCN from './locales/zh-CN.json'
import enUS from './locales/en-US.json'

export type MessageSchema = typeof zhCN
export type AppLocale = 'zh-CN' | 'en-US'

// 第一个类型参数 false 表示使用 Composition API 模式（legacy: false）
// 这样 TypeScript 可以正确推断 i18n.global 的类型为 Composer
const i18n = createI18n<
    false,
    {
        legacy: false
        locale: AppLocale
        fallbackLocale: AppLocale
        messages: Record<AppLocale, MessageSchema>
    }
>({
    legacy: false,
    locale: 'zh-CN',
    fallbackLocale: 'en-US',
    messages: {
        'zh-CN': zhCN,
        'en-US': enUS,
    },
})

export default i18n

// 导出类型化的全局对象
export const { t } = i18n.global

/**
 * 切换语言
 */
export function setI18nLanguage(locale: AppLocale): void {
    i18n.global.locale.value = locale
    document.documentElement.setAttribute('lang', locale)
}

/**
 * 获取当前语言
 */
export function getCurrentLocale(): AppLocale {
    return i18n.global.locale.value
}
