<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import SettingsGroup from '@/components/settings/SettingsGroup.vue'
import ThemeSelector from '@/components/settings/ThemeSelector.vue'
import LanguageSelector from '@/components/settings/LanguageSelector.vue'
import { useSettingsStore } from '@/stores/settings'
import { useTransferStore } from '@/stores/transfer'
import { computed, ref } from 'vue'

const { t } = useI18n()
const settingsStore = useSettingsStore()
const transferStore = useTransferStore()

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

// 立即清理
async function handleCleanupNow() {
    const strategy = cleanupStrategy.value
    if (strategy === 'disabled') return

    await transferStore.applyAutoCleanup(strategy, {
        retentionDays: retentionDays.value,
        maxCount: maxCount.value,
    })
}
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
                                    {{ t('common.edit', '编辑') }}
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

                <v-divider />

                <!-- 恢复默认设备名 -->
                <div class="d-flex align-center justify-space-between">
                    <span class="text-body-2 text-grey">
                        {{ t('settings.device.deviceName.reset') }}
                    </span>
                    <v-btn
                        variant="text"
                        color="primary"
                        size="small"
                        @click="resetDeviceName"
                    >
                        {{ t('settings.device.deviceName.reset') }}
                    </v-btn>
                </div>
            </div>
        </SettingsGroup>

        <SettingsGroup :title="t('settings.appearance')" class="mt-6">
            <div class="d-flex flex-column ga-4">
                <ThemeSelector />
                <v-divider />
                <LanguageSelector />
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

                <v-divider />

                <!-- 当前记录数和立即清理 -->
                <div class="d-flex align-center justify-space-between">
                    <span class="text-body-2 text-grey">
                        {{
                            t('settings.history.currentCount', {
                                count: transferStore.historyCount,
                            })
                        }}
                    </span>
                    <v-btn
                        v-if="cleanupStrategy !== 'disabled'"
                        variant="text"
                        color="primary"
                        size="small"
                        @click="handleCleanupNow"
                    >
                        {{ t('settings.history.cleanupNow') }}
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
</style>
