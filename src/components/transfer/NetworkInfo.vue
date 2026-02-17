<!-- 网络信息展示组件 -->
<template>
    <v-card class="network-info">
        <v-card-title class="text-subtitle-1 d-flex align-center">
            <v-icon :icon="mdiWifi" class="mr-2" color="primary" />
            网络信息
        </v-card-title>
        <v-card-text>
            <!-- IP 地址和端口 -->
            <div class="network-address mb-4">
                <div class="text-body-2 text-grey mb-2">监听地址</div>
                <v-text-field
                    :model-value="displayAddress"
                    readonly
                    variant="outlined"
                    density="compact"
                    :append-icon="mdiContentCopy"
                    @click:append="handleCopyAddress"
                />
            </div>

            <!-- 二维码 -->
            <div class="qr-code-section text-center mb-4">
                <div class="text-body-2 text-grey mb-2">扫码连接</div>
                <v-sheet
                    class="qr-code-container d-inline-flex align-center justify-center"
                    elevation="2"
                    rounded
                >
                    <v-img
                        v-if="qrCodeDataUrl"
                        :src="qrCodeDataUrl"
                        alt="连接二维码"
                        width="180"
                        height="180"
                        contain
                    />
                    <div v-else class="qr-code-placeholder">
                        <v-icon :icon="mdiQrcode" size="64" color="grey" />
                    </div>
                </v-sheet>
            </div>

            <!-- 分享码 -->
            <div class="share-code-section">
                <div class="text-body-2 text-grey mb-2">分享码</div>
                <v-chip
                    :text="shareCode"
                    color="primary"
                    variant="outlined"
                    label
                    class="mr-2"
                />
                <v-btn
                    :icon="mdiContentCopy"
                    size="small"
                    variant="text"
                    @click="handleCopyShareCode"
                />
            </div>

            <!-- 提示信息 -->
            <v-alert type="info" variant="tonal" density="compact" class="mt-4">
                其他设备可以通过扫描二维码或输入分享码连接到您的设备
            </v-alert>
        </v-card-text>

        <!-- 复制成功提示 -->
        <v-snackbar v-model="showCopySuccess" color="success" timeout="2000">
            已复制到剪贴板
        </v-snackbar>
    </v-card>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import QRCode from 'qrcode'
import { mdiWifi, mdiQrcode, mdiContentCopy } from '@mdi/js'

const props = defineProps<{
    networkAddress: string
    port: number
    shareCode: string
}>()

const showCopySuccess = ref(false)
const qrCodeDataUrl = ref<string>('')

const displayAddress = computed(() => {
    return `${props.networkAddress}:${props.port}`
})

const qrCodeData = computed(() => {
    return JSON.stringify({
        ip: props.networkAddress,
        port: props.port,
        code: props.shareCode,
    })
})

// 生成本地二维码
async function generateQRCode(): Promise<void> {
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
    [() => props.networkAddress, () => props.port, () => props.shareCode],
    () => {
        generateQRCode()
    },
    { immediate: true }
)

onMounted(() => {
    generateQRCode()
})

function handleCopyAddress() {
    navigator.clipboard.writeText(displayAddress.value)
    showCopySuccess.value = true
}

function handleCopyShareCode() {
    navigator.clipboard.writeText(props.shareCode)
    showCopySuccess.value = true
}
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
    align-items: center;
    justify-content: center;
    background: #f5f5f5;
    border-radius: 4px;
}

.share-code-section {
    display: flex;
    align-items: center;
    gap: 8px;
}
</style>
