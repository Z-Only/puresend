<!-- 传输模式切换组件 -->
<template>
    <v-card class="mode-switcher">
        <v-card-title class="text-subtitle-1 pb-0">
            {{ t('transfer.mode.title') }}
        </v-card-title>
        <v-card-text>
            <v-row>
                <v-col v-for="mode in modes" :key="mode.value" cols="6">
                    <v-tooltip :disabled="!mode.disabled" location="top">
                        <template #activator="{ props: tooltipProps }">
                            <v-card
                                v-bind="tooltipProps"
                                :color="
                                    modelValue === mode.value && !mode.disabled
                                        ? 'primary'
                                        : undefined
                                "
                                :variant="
                                    modelValue === mode.value && !mode.disabled
                                        ? 'flat'
                                        : 'outlined'
                                "
                                :class="[
                                    'mode-card',
                                    { 'mode-card-disabled': mode.disabled },
                                ]"
                                @click="selectMode(mode)"
                            >
                                <v-card-text
                                    class="d-flex flex-column align-center pa-4"
                                >
                                    <v-icon
                                        :icon="mode.icon"
                                        size="48"
                                        :color="
                                            modelValue === mode.value &&
                                            !mode.disabled
                                                ? 'white'
                                                : mode.disabled
                                                  ? 'grey'
                                                  : 'primary'
                                        "
                                        class="mb-2"
                                    />
                                    <div
                                        class="text-subtitle-1 font-weight-medium"
                                        :class="{ 'text-grey': mode.disabled }"
                                    >
                                        {{ mode.title }}
                                    </div>
                                    <div
                                        class="text-body-2 text-center mt-1"
                                        :class="
                                            modelValue === mode.value &&
                                            !mode.disabled
                                                ? 'text-white'
                                                : 'text-grey'
                                        "
                                    >
                                        {{ mode.description }}
                                    </div>
                                </v-card-text>
                            </v-card>
                        </template>
                        <span>{{ t('transfer.mode.cloudComingSoon') }}</span>
                    </v-tooltip>
                </v-col>
            </v-row>

            <!-- 本地网络提示 -->
            <v-alert
                v-if="modelValue === 'local' && onlinePeerCount === 0"
                type="info"
                variant="tonal"
                class="mt-4"
                density="compact"
            >
                {{ t('transfer.mode.noDeviceHint') }}
            </v-alert>
        </v-card-text>
    </v-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { TransferMode } from '../../types'
import { mdiWifi, mdiCloudUpload } from '@mdi/js'

const { t } = useI18n()

defineProps<{
    modelValue: TransferMode
    onlinePeerCount: number
}>()

const emit = defineEmits<{
    (e: 'update:modelValue', value: TransferMode): void
}>()

interface ModeOption {
    value: TransferMode
    title: string
    description: string
    icon: string
    disabled: boolean
}

const modes: ModeOption[] = [
    {
        value: 'local',
        title: t('transfer.mode.local.title'),
        description: t('transfer.mode.local.description'),
        icon: mdiWifi,
        disabled: false,
    },
    {
        value: 'cloud',
        title: t('transfer.mode.cloud.title'),
        description: t('transfer.mode.cloud.description'),
        icon: mdiCloudUpload,
        disabled: true,
    },
]

function selectMode(mode: ModeOption) {
    // 云盘中转模式禁用，不切换
    if (mode.disabled) {
        return
    }
    emit('update:modelValue', mode.value)
}
</script>

<style scoped src="./mode-selector-base.css"></style>
