/**
 * 字体大小 composable
 */
import { computed, watch } from 'vue'
import type { FontSizePreset } from '@/types/settings'
import { useSettingsStore } from '@/stores/settings'

/** 预设字体大小映射表 */
const PRESET_SCALE_MAP: Record<FontSizePreset, number> = {
    small: 0.85,
    medium: 1.0,
    large: 1.15,
    xlarge: 1.3,
}

/** 最小缩放比例 */
const MIN_SCALE = 0.8
/** 最大缩放比例 */
const MAX_SCALE = 1.5

export function useFontSize() {
    const settingsStore = useSettingsStore()

    /**
     * 获取系统字体大小设置
     */
    function getSystemFontScale(): number {
        try {
            // 检测系统字体大小偏好（通过媒体查询）
            const isLargeText = window.matchMedia(
                '(prefers-reduced-motion: reduce)'
            ).matches
            // 如果用户启了"减少动态效果"，通常也意味着需要更大的字体
            return isLargeText ? 1.2 : 1.0
        } catch (error) {
            console.error('[FontSize] 系统字体大小检测失败:', error)
            return 1.0
        }
    }

    /**
     * 计算实际缩放比例
     */
    const actualScale = computed((): number => {
        const { mode, preset, customScale } = settingsStore.fontSize

        let scale: number

        switch (mode) {
            case 'system':
                scale = getSystemFontScale()
                break
            case 'preset':
                scale = PRESET_SCALE_MAP[preset as FontSizePreset]
                break
            case 'custom':
                scale = customScale
                break
            default:
                scale = 1.0
        }

        // 限制缩放范围
        return Math.max(MIN_SCALE, Math.min(MAX_SCALE, scale))
    })

    /**
     * 应用字体大小到 DOM
     */
    function applyFontSize(): void {
        try {
            const scale = actualScale.value
            // 使用 rem 单位，通过修改根元素的 font-size 来实现全局缩放
            // 默认浏览器 font-size 为 16px，这里使用 calc() 来计算
            document.documentElement.style.fontSize = `calc(16px * ${scale})`
        } catch (error) {
            console.error('[FontSize] 应用字体大小失败:', error)
        }
    }

    /**
     * 监听字体大小设置变化，自动应用
     */
    watch(
        () => settingsStore.fontSize,
        () => {
            applyFontSize()
        },
        { deep: true, immediate: true }
    )

    return {
        actualScale,
        applyFontSize,
    }
}
