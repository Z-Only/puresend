<!-- 网络信息展示组件 -->
<template>
    <div class="network-info">
        <!-- 未接收状态提示 -->
        <v-alert
            v-if="!isReceiving"
            type="info"
            variant="tonal"
            density="compact"
            class="mb-4"
        >
            {{ t('network.notReceivingHint') }}
        </v-alert>

        <!-- 设备名称 -->
        <div class="info-field mb-4">
            <div class="text-body-2 text-grey mb-2">
                {{ t('network.deviceName') }}
            </div>
            <v-text-field
                :model-value="displayDeviceName"
                readonly
                variant="outlined"
                density="compact"
                hide-details
            />
        </div>

        <!-- IP 地址 -->
        <div class="info-field mb-4">
            <div class="text-body-2 text-grey mb-2">
                {{ t('network.ipAddress') }}
            </div>
            <v-text-field
                :model-value="networkAddress || '--'"
                readonly
                variant="outlined"
                density="compact"
                hide-details
            />
        </div>

        <!-- 端口 -->
        <div class="info-field mb-4">
            <div class="text-body-2 text-grey mb-2">
                {{ t('network.port') }}
            </div>
            <v-text-field
                :model-value="displayPort"
                readonly
                variant="outlined"
                density="compact"
                :disabled="!isReceiving"
                hide-details
            />
        </div>

        <!-- 二维码 - 仅在接收状态下显示 -->
        <div v-if="isReceiving" class="qr-code-section text-center mb-4">
            <div class="text-body-2 text-grey mb-2">
                {{ t('network.scanToConnect') }}
            </div>
            <v-sheet
                class="qr-code-container d-inline-flex align-center justify-center"
                elevation="2"
                rounded
            >
                <v-img
                    v-if="qrCodeDataUrl"
                    :src="qrCodeDataUrl"
                    :alt="t('network.qrCodeAlt')"
                    width="180"
                    height="180"
                    contain
                />
                <div v-else class="qr-code-placeholder">
                    <v-icon :icon="mdiQrcode" size="64" color="grey" />
                </div>
            </v-sheet>
        </div>

        <!-- 未接收时的占位提示 -->
        <div v-else class="qr-code-section text-center mb-4">
            <v-sheet
                class="qr-code-container d-inline-flex align-center justify-center"
                elevation="2"
                rounded
            >
                <div class="qr-code-placeholder">
                    <v-icon :icon="mdiQrcode" size="64" color="grey" />
                    <div class="text-body-2 text-grey mt-2">
                        {{ t('network.qrCodeUnavailable') }}
                    </div>
                </div>
            </v-sheet>
        </div>

        <!-- 提示信息 -->
        <v-alert type="info" variant="tonal" density="compact" class="mt-4">
            {{
                isReceiving
                    ? t('network.connectionHint')
                    : t('network.connectionHintInactive')
            }}
        </v-alert>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import QRCode from 'qrcode'
import { mdiQrcode } from '@mdi/js'

const { t } = useI18n()

const props = defineProps<{
    deviceName?: string
    networkAddress: string
    port: number
    isReceiving?: boolean
}>()

const qrCodeDataUrl = ref<string>('')

// 显示设备名称：始终显示实际名称
const displayDeviceName = computed(() => {
    return props.deviceName || '--'
})

// 显示端口：接收时显示实际端口，未接收时显示无数据
const displayPort = computed(() => {
    if (!props.isReceiving) {
        return '--'
    }
    return props.port ? props.port.toString() : '--'
})

const qrCodeData = computed(() => {
    if (!props.networkAddress || !props.port || !props.isReceiving) {
        return ''
    }
    return JSON.stringify({
        ip: props.networkAddress,
        port: props.port,
    })
})

// 生成本地二维码
async function generateQRCode(): Promise<void> {
    if (!props.isReceiving || !qrCodeData.value) {
        qrCodeDataUrl.value = ''
        return
    }

    try {
        qrCodeDataUrl.value = await QRCode.toDataURL(qrCodeData.value, {
            width: 180,
            margin: 2,
            errorCorrectionLevel: 'M',
        })
    } catch (error) {
        console.error('二维码生成失败:', error)
        qrCodeDataUrl.value = ''
    }
}

// 监听数据变化，重新生成二维码
watch(
    [() => props.networkAddress, () => props.port, () => props.isReceiving],
    () => {
        generateQRCode()
    },
    { immediate: true }
)
</script>

<style scoped>
.network-info {
    max-width: 400px;
}

.qr-code-container {
    background: white;
    padding: 16px;
    border-radius: 8px;
}

.qr-code-placeholder {
    width: 180px;
    height: 180px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background: #f5f5f5;
    border-radius: 4px;
}
</style>
