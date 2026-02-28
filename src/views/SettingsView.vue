<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import SettingsGroup from '@/components/settings/SettingsGroup.vue'
import ThemeSelector from '@/components/settings/ThemeSelector.vue'
import LanguageSelector from '@/components/settings/LanguageSelector.vue'
import { usePlatform } from '@/composables'
import { useSettingsStore } from '@/stores/settings'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, onMounted } from 'vue'
import type { PortRange } from '@/types/settings'

// 高级设置开关
const showAdvancedSettings = computed({
    get: () => settingsStore.showAdvancedSettings,
    set: (value) => settingsStore.setShowAdvancedSettings(value),
})

const { t } = useI18n()
const router = useRouter()
const { isMobile } = usePlatform()
const settingsStore = useSettingsStore()

// 设备名称编辑
const editingDeviceName = ref('')
const isEditingDeviceName = ref(false)

const deviceName = computed({
    get: () => settingsStore.deviceName,
    set: (value) => {
        editingDeviceName.value = value
    },
})

function startEditDeviceName() {
    editingDeviceName.value = settingsStore.deviceName
    isEditingDeviceName.value = true
}

async function saveDeviceName() {
    const trimmedName = editingDeviceName.value.trim()
    if (trimmedName && trimmedName !== settingsStore.deviceName) {
        await settingsStore.setDeviceName(trimmedName)
    }
    isEditingDeviceName.value = false
}

function cancelEditDeviceName() {
    editingDeviceName.value = ''
    isEditingDeviceName.value = false
}

async function resetDeviceName() {
    const systemName = await settingsStore.getSystemDeviceName()
    await settingsStore.setDeviceName(systemName)
}

// 是否记录历史
const recordHistory = computed({
    get: () => settingsStore.history.recordHistory,
    set: (value) => settingsStore.setRecordHistory(value),
})

// 隐私模式设置
const privacyEnabled = computed({
    get: () => settingsStore.history.privacy.enabled,
    set: (value) => settingsStore.setHistoryPrivacy({ enabled: value }),
})

const hideFileName = computed({
    get: () => settingsStore.history.privacy.hideFileName,
    set: (value) => settingsStore.setHistoryPrivacy({ hideFileName: value }),
})

const hidePeerName = computed({
    get: () => settingsStore.history.privacy.hidePeerName,
    set: (value) => settingsStore.setHistoryPrivacy({ hidePeerName: value }),
})

const hideIp = computed({
    get: () => settingsStore.history.privacy.hideIp,
    set: (value) => settingsStore.setHistoryPrivacy({ hideIp: value }),
})

// 自动清理设置
const cleanupStrategy = computed({
    get: () => settingsStore.history.autoCleanup.strategy,
    set: (value) => settingsStore.setAutoCleanup({ strategy: value }),
})

const retentionDays = computed({
    get: () => settingsStore.history.autoCleanup.retentionDays ?? 30,
    set: (value) => {
        // 确保值在有效范围内（最小1天）
        const validValue = Math.max(1, Math.floor(value) || 1)
        settingsStore.setAutoCleanup({ retentionDays: validValue })
    },
})

const maxCount = computed({
    get: () => settingsStore.history.autoCleanup.maxCount ?? 1000,
    set: (value) => {
        // 确保值在有效范围内（最小1条）
        const validValue = Math.max(1, Math.floor(value) || 1)
        settingsStore.setAutoCleanup({ maxCount: validValue })
    },
})

// 清理策略选项
const strategyOptions = computed(() => [
    {
        title: t('settings.history.autoCleanup.strategy.disabled'),
        value: 'disabled',
    },
    {
        title: t('settings.history.autoCleanup.strategy.byTime'),
        value: 'byTime',
    },
    {
        title: t('settings.history.autoCleanup.strategy.byCount'),
        value: 'byCount',
    },
])

// Tab 栏布局
const tabLayoutOptions = computed(() => [
    { title: t('settings.tabLayout.horizontalTop'), value: 'horizontal-top' },
    {
        title: t('settings.tabLayout.horizontalBottom'),
        value: 'horizontal-bottom',
    },
    { title: t('settings.tabLayout.verticalLeft'), value: 'vertical-left' },
    { title: t('settings.tabLayout.verticalRight'), value: 'vertical-right' },
])

const tabLayout = computed({
    get: () => settingsStore.tabLayout,
    set: (value) => settingsStore.setTabLayout(value),
})

// 字体大小设置
const fontSizeMode = computed({
    get: () => settingsStore.fontSize.mode,
    set: (value) => settingsStore.setFontSizeMode(value),
})

const fontSizePreset = computed({
    get: () => settingsStore.fontSize.preset,
    set: (value) => settingsStore.setFontSizePreset(value),
})

const fontSizeCustomScale = computed({
    get: () => settingsStore.fontSize.customScale,
    set: (value) => {
        // 确保值在有效范围内（0.8 - 1.5）
        const validValue = Math.max(0.8, Math.min(1.5, value))
        settingsStore.setFontSizeCustomScale(validValue)
    },
})

const fontSizeModeOptions = computed(() => [
    { title: t('settings.fontSize.mode.system'), value: 'system' },
    { title: t('settings.fontSize.mode.preset'), value: 'preset' },
    { title: t('settings.fontSize.mode.custom'), value: 'custom' },
])

const fontSizePresetOptions = computed(() => [
    { title: t('settings.fontSize.preset.small'), value: 'small' },
    { title: t('settings.fontSize.preset.medium'), value: 'medium' },
    { title: t('settings.fontSize.preset.large'), value: 'large' },
    { title: t('settings.fontSize.preset.xlarge'), value: 'xlarge' },
])

// 开发者设置
const devToolsEnabled = computed({
    get: () => settingsStore.developerSettings.devToolsEnabled,
    set: async (value) => {
        await settingsStore.setDevToolsEnabled(value)
        try {
            await invoke('toggle_devtools', { enabled: value })
        } catch {
            // Tauri 不可用时忽略
        }
    },
})

// 端口范围配置
const transferPortMin = ref(0)
const transferPortMax = ref(0)
const webUploadPortMin = ref(0)
const webUploadPortMax = ref(0)
const sharePortMin = ref(0)
const sharePortMax = ref(0)

function loadPortRangeValues() {
    const portRange = settingsStore.developerSettings.portRange
    transferPortMin.value = portRange.transfer.minPort
    transferPortMax.value = portRange.transfer.maxPort
    webUploadPortMin.value = portRange.webUpload.minPort
    webUploadPortMax.value = portRange.webUpload.maxPort
    sharePortMin.value = portRange.share.minPort
    sharePortMax.value = portRange.share.maxPort
}

onMounted(() => {
    loadPortRangeValues()
})

function normalizePortValue(value: number): number {
    if (!Number.isInteger(value) || isNaN(value)) return 0
    if (value === 0) return 0
    if (value < 1024) return 1024
    if (value > 65535) return 65535
    return value
}

function normalizePortRange(minPort: number, maxPort: number): PortRange {
    let normalizedMin = normalizePortValue(minPort)
    let normalizedMax = normalizePortValue(maxPort)
    if (
        normalizedMin !== 0 &&
        normalizedMax !== 0 &&
        normalizedMin > normalizedMax
    ) {
        const temp = normalizedMin
        normalizedMin = normalizedMax
        normalizedMax = temp
    }
    return { minPort: normalizedMin, maxPort: normalizedMax }
}

async function saveTransferPortRange() {
    const range = normalizePortRange(
        transferPortMin.value,
        transferPortMax.value
    )
    transferPortMin.value = range.minPort
    transferPortMax.value = range.maxPort
    await settingsStore.setPortRange('transfer', range)
}

async function saveWebUploadPortRange() {
    const range = normalizePortRange(
        webUploadPortMin.value,
        webUploadPortMax.value
    )
    webUploadPortMin.value = range.minPort
    webUploadPortMax.value = range.maxPort
    await settingsStore.setPortRange('webUpload', range)
}

async function saveSharePortRange() {
    const range = normalizePortRange(sharePortMin.value, sharePortMax.value)
    sharePortMin.value = range.minPort
    sharePortMax.value = range.maxPort
    await settingsStore.setPortRange('share', range)
}

// 应用版本
const appVersion = __APP_VERSION__

// 传输增强设置
const encryptionEnabled = computed({
    get: () => settingsStore.encryptionSettings.enabled,
    set: (value) => settingsStore.setEncryptionEnabled(value),
})

const compressionEnabled = computed({
    get: () => settingsStore.compressionSettings.enabled,
    set: (value) => settingsStore.setCompressionEnabled(value),
})

const compressionMode = computed({
    get: () => settingsStore.compressionSettings.mode,
    set: (value) => settingsStore.setCompressionMode(value),
})

const compressionLevel = computed({
    get: () => settingsStore.compressionSettings.level,
    set: (value) => settingsStore.setCompressionLevel(value),
})

const compressionModeOptions = computed(() => [
    {
        title: t('settings.transferEnhancement.compression.mode.smart'),
        value: 'smart',
    },
    {
        title: t('settings.transferEnhancement.compression.mode.manual'),
        value: 'manual',
    },
])
</script>

<template>
    <v-container class="settings-view" max-width="800">
        <h1 class="text-h4 mb-6">
            {{ t('settings.title') }}
        </h1>

        <!-- 设备设置 -->
        <SettingsGroup :title="t('settings.device.title')">
            <div class="d-flex flex-column ga-4">
                <!-- 设备名称 -->
                <div>
                    <div class="d-flex align-center justify-space-between">
                        <div>
                            <div class="text-subtitle-1">
                                {{ t('settings.device.deviceName.label') }}
                            </div>
                            <div class="text-body-2 text-grey">
                                {{
                                    t('settings.device.deviceName.description')
                                }}
                            </div>
                        </div>
                        <div class="d-flex align-center ga-2">
                            <template v-if="!isEditingDeviceName">
                                <span class="text-body-1">{{
                                    deviceName
                                }}</span>
                                <v-btn
                                    variant="text"
                                    color="primary"
                                    size="small"
                                    @click="startEditDeviceName"
                                >
                                    {{ t('common.edit') }}
                                </v-btn>
                                <v-btn
                                    variant="text"
                                    color="secondary"
                                    size="small"
                                    @click="resetDeviceName"
                                >
                                    {{ t('settings.device.deviceName.reset') }}
                                </v-btn>
                            </template>
                            <template v-else>
                                <v-text-field
                                    v-model="editingDeviceName"
                                    :placeholder="
                                        t(
                                            'settings.device.deviceName.placeholder'
                                        )
                                    "
                                    density="compact"
                                    variant="outlined"
                                    hide-details
                                    style="min-width: 240px"
                                    @keydown.enter="saveDeviceName"
                                    @keydown.escape="cancelEditDeviceName"
                                />
                                <v-btn
                                    variant="text"
                                    color="primary"
                                    size="small"
                                    @click="saveDeviceName"
                                >
                                    {{ t('common.confirm') }}
                                </v-btn>
                                <v-btn
                                    variant="text"
                                    size="small"
                                    @click="cancelEditDeviceName"
                                >
                                    {{ t('common.cancel') }}
                                </v-btn>
                            </template>
                        </div>
                    </div>
                </div>
            </div>
        </SettingsGroup>

        <SettingsGroup :title="t('settings.appearance')" class="mt-6">
            <div class="d-flex flex-column ga-4">
                <ThemeSelector />
                <v-divider />
                <LanguageSelector />
                <v-divider />
                <!-- 字体大小设置 -->
                <div class="d-flex flex-column ga-4">
                    <div class="d-flex align-center justify-space-between">
                        <div>
                            <div class="text-subtitle-1">
                                {{ t('settings.fontSize.label') }}
                            </div>
                        </div>
                        <v-select
                            v-model="fontSizeMode"
                            :items="fontSizeModeOptions"
                            density="compact"
                            variant="outlined"
                            hide-details
                            style="max-width: 200px"
                        />
                    </div>
                    <!-- 预设大小选项 -->
                    <div
                        v-if="fontSizeMode === 'preset'"
                        class="d-flex align-center justify-space-between"
                    >
                        <div>
                            <div class="text-subtitle-1">
                                {{ t('settings.fontSize.mode.preset') }}
                            </div>
                        </div>
                        <v-select
                            v-model="fontSizePreset"
                            :items="fontSizePresetOptions"
                            density="compact"
                            variant="outlined"
                            hide-details
                            style="max-width: 200px"
                        />
                    </div>
                    <!-- 自定义缩放滑块 -->
                    <div
                        v-if="fontSizeMode === 'custom'"
                        class="d-flex flex-column ga-2"
                    >
                        <div class="d-flex align-center justify-space-between">
                            <div class="text-subtitle-1">
                                {{ t('settings.fontSize.custom.label') }}
                            </div>
                            <span class="text-body-2">
                                {{ Math.round(fontSizeCustomScale * 100) }}%
                            </span>
                        </div>
                        <v-slider
                            v-model="fontSizeCustomScale"
                            :min="0.8"
                            :max="1.5"
                            :step="0.05"
                            color="primary"
                            hide-details
                            density="compact"
                        >
                            <template #prepend>
                                <span class="text-body-2 text-grey">
                                    {{ t('settings.fontSize.custom.min') }}
                                </span>
                            </template>
                            <template #append>
                                <span class="text-body-2 text-grey">
                                    {{ t('settings.fontSize.custom.max') }}
                                </span>
                            </template>
                        </v-slider>
                    </div>
                </div>
                <template v-if="!isMobile">
                    <v-divider />
                    <div class="d-flex align-center justify-space-between">
                        <div>
                            <div class="text-subtitle-1">
                                {{ t('settings.tabLayout.label') }}
                            </div>
                        </div>
                        <v-select
                            v-model="tabLayout"
                            :items="tabLayoutOptions"
                            density="compact"
                            variant="outlined"
                            hide-details
                            style="max-width: 200px"
                        />
                    </div>
                </template>
            </div>
        </SettingsGroup>

        <!-- 历史记录设置 -->
        <SettingsGroup :title="t('settings.history.title')" class="mt-6">
            <div class="d-flex flex-column ga-4">
                <!-- 是否记录历史 -->
                <div class="d-flex align-center justify-space-between">
                    <div>
                        <div class="text-subtitle-1">
                            {{ t('settings.history.recordHistory.label') }}
                        </div>
                        <div class="text-body-2 text-grey">
                            {{
                                t('settings.history.recordHistory.description')
                            }}
                        </div>
                    </div>
                    <v-switch
                        v-model="recordHistory"
                        color="primary"
                        hide-details
                    />
                </div>

                <v-divider />

                <!-- 隐私模式 -->
                <div>
                    <div class="d-flex align-center justify-space-between">
                        <div>
                            <div class="text-subtitle-1">
                                {{ t('settings.history.privacy.label') }}
                            </div>
                            <div class="text-body-2 text-grey">
                                {{ t('settings.history.privacy.description') }}
                            </div>
                        </div>
                        <v-switch
                            v-model="privacyEnabled"
                            color="primary"
                            hide-details
                        />
                    </div>

                    <v-expand-transition>
                        <div v-if="privacyEnabled" class="ml-4 mt-2">
                            <v-checkbox
                                v-model="hideFileName"
                                :label="
                                    t('settings.history.privacy.hideFileName')
                                "
                                density="compact"
                                hide-details
                            />
                            <v-checkbox
                                v-model="hidePeerName"
                                :label="
                                    t('settings.history.privacy.hidePeerName')
                                "
                                density="compact"
                                hide-details
                            />
                            <v-checkbox
                                v-model="hideIp"
                                :label="t('settings.history.privacy.hideIp')"
                                density="compact"
                                hide-details
                            />
                        </div>
                    </v-expand-transition>
                </div>

                <v-divider />

                <!-- 自动清理 -->
                <div>
                    <div class="text-subtitle-1">
                        {{ t('settings.history.autoCleanup.label') }}
                    </div>
                    <div class="text-body-2 text-grey mb-2">
                        {{ t('settings.history.autoCleanup.description') }}
                    </div>

                    <v-select
                        v-model="cleanupStrategy"
                        :items="strategyOptions"
                        density="compact"
                        variant="outlined"
                        hide-details
                        class="mb-2"
                    />

                    <v-expand-transition>
                        <div
                            v-if="cleanupStrategy === 'byTime'"
                            class="d-flex align-center ga-2"
                        >
                            <v-text-field
                                v-model.number="retentionDays"
                                type="number"
                                :min="1"
                                density="compact"
                                variant="outlined"
                                hide-details
                                style="max-width: 100px"
                            />
                            <span class="text-body-2">{{
                                t('settings.history.autoCleanup.daysUnit')
                            }}</span>
                        </div>
                    </v-expand-transition>

                    <v-expand-transition>
                        <div
                            v-if="cleanupStrategy === 'byCount'"
                            class="d-flex align-center ga-2"
                        >
                            <v-text-field
                                v-model.number="maxCount"
                                type="number"
                                :min="1"
                                density="compact"
                                variant="outlined"
                                hide-details
                                style="max-width: 100px"
                            />
                            <span class="text-body-2">{{
                                t('settings.history.autoCleanup.countUnit')
                            }}</span>
                        </div>
                    </v-expand-transition>
                </div>
            </div>
        </SettingsGroup>

        <!-- 高级设置开关 -->
        <div class="d-flex justify-end mt-6 mb-2">
            <v-checkbox
                v-model="showAdvancedSettings"
                :label="t('settings.advanced.showLabel')"
                density="compact"
                hide-details
                color="primary"
            />
        </div>

        <!-- 传输增强设置 -->
        <SettingsGroup
            v-if="showAdvancedSettings"
            :title="t('settings.transferEnhancement.title')"
            class="mt-2"
        >
            <div class="d-flex flex-column ga-4">
                <!-- 传输加密 -->
                <div class="d-flex align-center justify-space-between">
                    <div>
                        <div class="text-subtitle-1">
                            {{
                                t(
                                    'settings.transferEnhancement.encryption.label'
                                )
                            }}
                        </div>
                        <div class="text-body-2 text-grey">
                            {{
                                t(
                                    'settings.transferEnhancement.encryption.description'
                                )
                            }}
                        </div>
                    </div>
                    <v-switch
                        v-model="encryptionEnabled"
                        color="primary"
                        hide-details
                    />
                </div>
                <v-divider />
                <!-- 动态压缩 -->
                <div class="d-flex align-center justify-space-between">
                    <div>
                        <div class="text-subtitle-1">
                            {{
                                t(
                                    'settings.transferEnhancement.compression.label'
                                )
                            }}
                        </div>
                        <div class="text-body-2 text-grey">
                            {{
                                t(
                                    'settings.transferEnhancement.compression.description'
                                )
                            }}
                        </div>
                    </div>
                    <v-switch
                        v-model="compressionEnabled"
                        color="primary"
                        hide-details
                    />
                </div>
                <v-expand-transition>
                    <div
                        v-if="compressionEnabled"
                        class="d-flex flex-column ga-4"
                    >
                        <div class="d-flex align-center justify-space-between">
                            <div class="text-subtitle-1">
                                {{
                                    t(
                                        'settings.transferEnhancement.compression.mode.label'
                                    )
                                }}
                            </div>
                            <v-select
                                v-model="compressionMode"
                                :items="compressionModeOptions"
                                density="compact"
                                variant="outlined"
                                hide-details
                                style="max-width: 200px"
                            />
                        </div>
                        <div
                            v-if="compressionMode === 'smart'"
                            class="text-body-2 text-grey"
                        >
                            {{
                                t(
                                    'settings.transferEnhancement.compression.mode.smartDescription'
                                )
                            }}
                        </div>
                        <v-expand-transition>
                            <div
                                v-if="compressionMode === 'manual'"
                                class="d-flex flex-column ga-2"
                            >
                                <div
                                    class="d-flex align-center justify-space-between"
                                >
                                    <div class="text-subtitle-1">
                                        {{
                                            t(
                                                'settings.transferEnhancement.compression.level.label'
                                            )
                                        }}
                                    </div>
                                    <span class="text-body-2">{{
                                        compressionLevel
                                    }}</span>
                                </div>
                                <v-slider
                                    v-model="compressionLevel"
                                    :min="1"
                                    :max="19"
                                    :step="1"
                                    color="primary"
                                    hide-details
                                    density="compact"
                                >
                                    <template #prepend
                                        ><span class="text-body-2 text-grey">{{
                                            t(
                                                'settings.transferEnhancement.compression.level.low'
                                            )
                                        }}</span></template
                                    >
                                    <template #append
                                        ><span class="text-body-2 text-grey">{{
                                            t(
                                                'settings.transferEnhancement.compression.level.high'
                                            )
                                        }}</span></template
                                    >
                                </v-slider>
                            </div>
                        </v-expand-transition>
                    </div>
                </v-expand-transition>
            </div>
        </SettingsGroup>

        <!-- 开发者设置 -->
        <SettingsGroup
            v-if="showAdvancedSettings"
            :title="t('settings.developer.title')"
            class="mt-2"
        >
            <div class="d-flex flex-column ga-4">
                <!-- DevTools 开关 -->
                <div class="d-flex align-center justify-space-between">
                    <div>
                        <div class="text-subtitle-1">
                            {{ t('settings.developer.devTools.label') }}
                        </div>
                        <div class="text-body-2 text-grey">
                            {{ t('settings.developer.devTools.description') }}
                        </div>
                    </div>
                    <v-switch
                        v-model="devToolsEnabled"
                        color="primary"
                        hide-details
                    />
                </div>

                <!-- Android DevTools 提示 -->
                <v-alert
                    v-if="isMobile && devToolsEnabled"
                    type="info"
                    variant="tonal"
                    density="compact"
                >
                    {{ t('settings.developer.devTools.androidHint') }}
                </v-alert>

                <v-divider />

                <!-- 端口范围配置 -->
                <div>
                    <div class="text-subtitle-1 mb-2">
                        {{ t('settings.developer.portRange.label') }}
                    </div>
                    <div class="text-body-2 text-grey mb-4">
                        {{ t('settings.developer.portRange.hint') }}
                    </div>

                    <!-- 文件接收服务器 -->
                    <div class="port-range-item mb-4">
                        <div class="port-range-label">
                            <div class="text-body-2 font-weight-medium">
                                {{ t('settings.developer.portRange.transfer') }}
                            </div>
                        </div>
                        <div class="port-range-inputs">
                            <v-text-field
                                v-model.number="transferPortMin"
                                type="number"
                                :min="0"
                                :max="65535"
                                :label="
                                    t('settings.developer.portRange.minPort')
                                "
                                density="compact"
                                variant="outlined"
                                hide-details
                                class="port-input"
                                @blur="saveTransferPortRange"
                            />
                            <span class="port-separator">—</span>
                            <v-text-field
                                v-model.number="transferPortMax"
                                type="number"
                                :min="0"
                                :max="65535"
                                :label="
                                    t('settings.developer.portRange.maxPort')
                                "
                                density="compact"
                                variant="outlined"
                                hide-details
                                class="port-input"
                                @blur="saveTransferPortRange"
                            />
                        </div>
                    </div>

                    <!-- HTTP 上传服务器 -->
                    <div class="port-range-item mb-4">
                        <div class="port-range-label">
                            <div class="text-body-2 font-weight-medium">
                                {{
                                    t('settings.developer.portRange.webUpload')
                                }}
                            </div>
                        </div>
                        <div class="port-range-inputs">
                            <v-text-field
                                v-model.number="webUploadPortMin"
                                type="number"
                                :min="0"
                                :max="65535"
                                :label="
                                    t('settings.developer.portRange.minPort')
                                "
                                density="compact"
                                variant="outlined"
                                hide-details
                                class="port-input"
                                @blur="saveWebUploadPortRange"
                            />
                            <span class="port-separator">—</span>
                            <v-text-field
                                v-model.number="webUploadPortMax"
                                type="number"
                                :min="0"
                                :max="65535"
                                :label="
                                    t('settings.developer.portRange.maxPort')
                                "
                                density="compact"
                                variant="outlined"
                                hide-details
                                class="port-input"
                                @blur="saveWebUploadPortRange"
                            />
                        </div>
                    </div>

                    <!-- HTTP 下载服务器 -->
                    <div class="port-range-item">
                        <div class="port-range-label">
                            <div class="text-body-2 font-weight-medium">
                                {{ t('settings.developer.portRange.share') }}
                            </div>
                        </div>
                        <div class="port-range-inputs">
                            <v-text-field
                                v-model.number="sharePortMin"
                                type="number"
                                :min="0"
                                :max="65535"
                                :label="
                                    t('settings.developer.portRange.minPort')
                                "
                                density="compact"
                                variant="outlined"
                                hide-details
                                class="port-input"
                                @blur="saveSharePortRange"
                            />
                            <span class="port-separator">—</span>
                            <v-text-field
                                v-model.number="sharePortMax"
                                type="number"
                                :min="0"
                                :max="65535"
                                :label="
                                    t('settings.developer.portRange.maxPort')
                                "
                                ler
                                density="compact"
                                variant="outlined"
                                hide-details
                                class="port-input"
                                @blur="saveSharePortRange"
                            />
                        </div>
                    </div>
                </div>
            </div>
        </SettingsGroup>

        <!-- 关于 -->
        <SettingsGroup :title="t('settings.about.title')" class="mt-6">
            <div class="d-flex flex-column align-center py-4">
                <v-avatar size="96" class="mb-3">
                    <v-img src="/icons/icon.png" alt="PureSend" />
                </v-avatar>
                <div class="text-h6 font-weight-bold">PureSend</div>
                <div class="text-body-2 text-grey mt-1">
                    {{ t('settings.about.version') }} {{ appVersion }}
                </div>
                <div class="d-flex flex-column ga-1 mt-2">
                    <v-btn
                        variant="text"
                        color="primary"
                        size="small"
                        class="about-link-btn"
                        @click="router.push('/changelog')"
                    >
                        {{ t('settings.about.changelog') }}
                    </v-btn>
                    <v-btn
                        variant="text"
                        color="primary"
                        size="small"
                        class="about-link-btn"
                        href="https://puresend.vercel.app"
                        target="_blank"
                    >
                        {{ t('settings.about.homepage') }}
                    </v-btn>
                    <v-btn
                        variant="text"
                        color="primary"
                        size="small"
                        class="about-link-btn"
                        href="https://github.com/z-only/puresend"
                        target="_blank"
                    >
                        {{ t('settings.about.github') }}
                    </v-btn>
                </div>
            </div>
        </SettingsGroup>
    </v-container>
</template>

<style scoped>
.settings-view {
    padding: 24px;
}

.about-link-btn {
    font-size: 0.875rem !important;
    letter-spacing: normal !important;
}

/* 端口范围配置响应式布局 */
.port-range-item {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 16px;
    padding-right: 8px;
}

.port-range-item:last-child {
    margin-bottom: 0;
}

.port-range-label {
    flex: 0 0 180px;
    min-width: 180px;
    padding-top: 8px;
}

.port-range-inputs {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    justify-content: flex-end;
    min-width: 0;
}

.port-input {
    flex: 0 0 100px;
    max-width: 100px;
}

.port-input :deep(.v-field__input) {
    font-family: 'Courier New', monospace;
    font-size: 0.875rem;
}

.port-separator {
    color: rgba(var(--v-theme-on-surface), 0.6);
    font-weight: 500;
    padding: 0 4px;
    flex-shrink: 0;
}

/* 响应式：小屏幕下换行显示 */
@media (max-width: 600px) {
    .port-range-item {
        flex-direction: column;
        gap: 8px;
    }

    .port-range-label {
        flex: none;
        min-width: auto;
        width: 100%;
        padding-top: 0;
        margin-bottom: 4px;
    }

    .port-range-inputs {
        width: 100%;
        flex-wrap: wrap;
    }

    .port-input {
        flex: 1;
        min-width: 80px;
        max-width: none;
    }
}
</style>
