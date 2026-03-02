<!-- 云盘中转页面 -->
<template>
    <v-container fluid class="cloud-view">
        <!-- 账号选择器 -->
        <v-card class="mb-4">
            <v-card-text>
                <v-select
                    v-model="selectedAccountId"
                    :label="t('cloudTransfer.selectAccount')"
                    :items="cloudStore.accounts"
                    item-title="name"
                    item-value="id"
                    :disabled="!cloudStore.hasAccounts"
                    variant="outlined"
                    :prepend-inner-icon="mdiCloud"
                    @update:model-value="handleAccountChange"
                >
                    <template #item="{ item, props }">
                        <v-list-item v-bind="props">
                            <template #prepend>
                                <v-icon :icon="mdiCloud" />
                            </template>
                            <template #append>
                                <v-chip
                                    :color="getStatusColor(item.status)"
                                    size="small"
                                    variant="flat"
                                >
                                    {{ getStatusText(item.status) }}
                                </v-chip>
                            </template>
                        </v-list-item>
                    </template>
                </v-select>

                <!-- 无账号提示 -->
                <v-alert
                    v-if="!cloudStore.hasAccounts"
                    type="info"
                    variant="tonal"
                >
                    {{ t('cloudTransfer.noAccounts') }}
                    <template #append>
                        <v-btn size="small" variant="text" to="/settings">
                            {{ t('settings.title') }}
                        </v-btn>
                    </template>
                </v-alert>
            </v-card-text>
        </v-card>

        <!-- 上传和下载面板 -->
        <v-row v-if="selectedAccountId">
            <!-- 上传面板 -->
            <v-col cols="12" md="6">
                <CloudUploadPanel :account-id="selectedAccountId" />
            </v-col>

            <!-- 下载面板 -->
            <v-col cols="12" md="6">
                <CloudDownloadPanel :account-id="selectedAccountId" />
            </v-col>
        </v-row>
    </v-container>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useCloudStore } from '@/stores/cloud'
import CloudUploadPanel from '@/components/cloud/CloudUploadPanel.vue'
import CloudDownloadPanel from '@/components/cloud/CloudDownloadPanel.vue'
import type { CloudAccountStatus } from '@/types/cloud'
import { mdiCloud } from '@mdi/js'

// 持久化存储的 key
const STORAGE_KEY = 'puresend_cloud_selected_account'

const { t } = useI18n()
const cloudStore = useCloudStore()

const selectedAccountId = ref<string>('')

onMounted(async () => {
    await cloudStore.loadAccounts()
    if (cloudStore.hasAccounts) {
        // 从 localStorage 恢复选中的账号
        const savedAccountId = localStorage.getItem(STORAGE_KEY)
        if (
            savedAccountId &&
            cloudStore.accounts.find((a) => a.id === savedAccountId)
        ) {
            selectedAccountId.value = savedAccountId
        } else {
            selectedAccountId.value = cloudStore.accounts[0].id
        }
    }
})

function handleAccountChange(accountId: string): void {
    // 持久化选中的账号
    if (accountId) {
        localStorage.setItem(STORAGE_KEY, accountId)
    }
}

function getStatusColor(status: CloudAccountStatus): string {
    const colorMap: Record<CloudAccountStatus, string> = {
        connected: 'success',
        disconnected: 'warning',
        invalid: 'error',
    }
    return colorMap[status] || 'grey'
}

function getStatusText(status: CloudAccountStatus): string {
    return t(`cloudAccount.status.${status}`)
}
</script>

<style scoped>
.cloud-view {
    max-width: 1400px;
    margin: 0 auto;
}
</style>
