<!-- 应用程序选择器组件 -->
<template>
    <div class="app-picker">
        <v-card variant="outlined" class="pa-4">
            <v-card-text>
                <div class="d-flex align-center mb-3">
                    <v-icon
                        :icon="mdiApplication"
                        class="mr-2"
                        color="primary"
                    />
                    <span class="text-subtitle-1 font-weight-bold">
                        {{ t('appPicker.title') }}
                    </span>
                </div>

                <div class="text-body-2 text-grey mb-3">
                    {{ t('appPicker.description') }}
                </div>

                <v-btn
                    color="primary"
                    :loading="loading"
                    block
                    class="text-center picker-btn"
                    @click="pickApp"
                >
                    <v-icon :icon="mdiApps" />
                    <span class="btn-text">{{ t('appPicker.selectApp') }}</span>
                </v-btn>

                <v-alert
                    v-if="errorMessage"
                    type="error"
                    variant="tonal"
                    class="mt-4"
                    density="compact"
                >
                    {{ errorMessage }}
                </v-alert>

                <!-- 已选择的应用信息 -->
                <div v-if="selectedApp" class="mt-4">
                    <v-divider class="mb-3" />
                    <div class="d-flex align-center">
                        <v-icon
                            :icon="mdiApplication"
                            size="40"
                            color="primary"
                            class="mr-3"
                        />
                        <div class="flex-grow-1">
                            <div class="text-subtitle-1">
                                {{ selectedApp.name }}
                            </div>
                            <div
                                v-if="selectedApp.metadata?.version"
                                class="text-body-2 text-grey"
                            >
                                {{
                                    t('appPicker.version', {
                                        version: selectedApp.metadata.version,
                                    })
                                }}
                            </div>
                            <div class="text-body-2 text-grey">
                                {{ selectedApp.path }}
                            </div>
                        </div>
                    </div>
                </div>
            </v-card-text>
        </v-card>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { open } from '@tauri-apps/plugin-dialog'
import { getFileMetadata } from '../../services/transferService'
import type { ContentItem } from '../../types'
import { getContentFilterNameKey } from '../../types'
import { mdiApplication, mdiApps } from '@mdi/js'

const { t } = useI18n()

const emit = defineEmits<{
    (e: 'select', item: ContentItem): void
}>()

const loading = ref(false)
const selectedApp = ref<ContentItem | null>(null)
const errorMessage = ref('')

async function pickApp() {
    loading.value = true
    errorMessage.value = ''
    try {
        // 根据平台选择不同的应用选择方式
        const selected = await open({
            multiple: false,
            filters: [
                {
                    name: t(
                        getContentFilterNameKey('app') || 'content.filter.app'
                    ),
                    extensions: ['app', 'exe', 'dmg', 'pkg', 'deb', 'rpm'],
                },
            ],
        })

        if (selected && typeof selected === 'string') {
            const name = selected.split(/[/\\]/).pop() || selected
            const extension = name.split('.').pop()?.toLowerCase() || ''

            // 获取应用元数据
            let size = 0
            let mimeType = getMimeType(extension)
            try {
                const metadata = await getFileMetadata(selected)
                size = metadata.size
                mimeType = metadata.mimeType
            } catch (metaError) {
                console.warn('获取应用元数据失败:', metaError)
            }

            selectedApp.value = {
                type: 'app',
                path: selected,
                name,
                size,
                mimeType,
                createdAt: Date.now(),
                metadata: {
                    platform: getPlatform(),
                },
            }

            emit('select', selectedApp.value)
        }
        // 用户取消选择时不显示错误信息，静默关闭即可
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)
        errorMessage.value = t('appPicker.selectFailed', { error: errorMsg })
        console.error('选择应用失败:', error)
    } finally {
        loading.value = false
    }
}

function getMimeType(extension: string): string {
    const mimeTypes: Record<string, string> = {
        app: 'application/x-macos-app',
        exe: 'application/x-msdownload',
        dmg: 'application/x-apple-diskimage',
        pkg: 'application/vnd.apple.installer-package',
        deb: 'application/vnd.debian.binary-package',
        rpm: 'application/x-rpm',
    }
    return mimeTypes[extension] || 'application/octet-stream'
}

function getPlatform(): string {
    const userAgent = navigator.userAgent.toLowerCase()
    if (userAgent.includes('mac')) return 'macos'
    if (userAgent.includes('win')) return 'windows'
    if (userAgent.includes('linux')) return 'linux'
    return 'unknown'
}
</script>

<style scoped>
.app-picker {
    width: 100%;
}

.picker-btn {
    display: grid !important;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    justify-items: center;
}

.picker-btn .v-icon {
    grid-column: 1;
}

.picker-btn .btn-text {
    grid-column: 2;
    text-align: center;
}
</style>
