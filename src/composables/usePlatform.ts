/**
 * 平台检测 composable
 */
import { ref, onMounted } from 'vue'

export function usePlatform() {
    const isMobile = ref(false)
    const isMacOS = ref(false)

    onMounted(async () => {
        try {
            const { platform } = await import('@tauri-apps/plugin-os')
            const currentPlatform = await platform()
            isMobile.value = currentPlatform === 'android' || currentPlatform === 'ios'
            isMacOS.value = currentPlatform === 'macos'
        } catch {
            // 降级：通过 userAgent 判断
            const userAgent = navigator.userAgent.toLowerCase()
            isMobile.value = /android|iphone|ipad|ipod/.test(userAgent)
            isMacOS.value = /macintosh|mac os x/.test(userAgent)
        }
    })

    return {
        isMobile,
        isMacOS,
    }
}
