<template>
    <v-card class="cloud-send-settings">
        <v-card-text>
            <!-- 账号选择器 -->
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
                class="mt-4"
            >
                {{ t('cloudTransfer.noAccounts') }}
                <template #append>
                    <v-btn size="small" variant="text" to="/settings">
                        {{ t('settings.title') }}
                    </v-btn>
                </template>
            </v-alert>

            <!-- 目标目录配置 -->
            <div v-if="selectedAccountId" class="mt-4">
                <v-label>{{ t('cloudTransfer.targetDirectory') }}</v-label>
                <div class="directory-input-wrapper mt-2">
                    <v-text-field
                        v-model="targetDirectory"
                        variant="outlined"
                        density="compact"
                        :prepend-inner-icon="mdiFolder"
                        readonly
                    ></v-text-field>
                    <v-btn
                        color="primary"
                        variant="outlined"
                        :prepend-icon="mdiFolderOpen"
                        @click="openDirectoryBrowser"
                    >
                        {{ t('cloudTransfer.browse') }}
                    </v-btn>
                </div>
            </div>
        </v-card-text>

        <!-- 目录浏览器对话框 -->
        <CloudDirectoryBrowser
            v-model="showDirectoryBrowser"
            :account-id="selectedAccountId"
            :initial-path="targetDirectory"
            mode="directory"
            @select="handleDirectorySelect"
        />
    </v-card>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useCloudStore } from '@/stores/cloud'
import CloudDirectoryBrowser from './CloudDirectoryBrowser.vue'
import type { CloudAccountStatus } from '@/types/cloud'
import { mdiCloud, mdiFolder, mdiFolderOpen } from '@mdi/js'

const emit = defineEmits<{
    accountSelected: [accountId: string]
    directorySelected: [path: string]
}>()

const { t } = useI18n()
const cloudStore = useCloudStore()

const selectedAccountId = ref<string>('')
const targetDirectory = ref<string>('')
const showDirectoryBrowser = ref<boolean>(false)

onMounted(async () => {
    await cloudStore.loadAccounts()
    if (cloudStore.hasAccounts) {
        selectedAccountId.value = cloudStore.accounts[0].id
        // 默认目录为云盘根目录
        targetDirectory.value = '/'
    }
})

function handleAccountChange(accountId: string): void {
    // 切换账号时，默认目录为云盘根目录
    targetDirectory.value = '/'
    emit('accountSelected', accountId)
}

function openDirectoryBrowser(): void {
    showDirectoryBrowser.value = true
}

function handleDirectorySelect(path: string): void {
    targetDirectory.value = path
    emit('directorySelected', path)
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
    const textMap: Record<CloudAccountStatus, string> = {
        connected: t('common.enabled'),
        disconnected: t('common.disabled'),
        invalid: 'invalid',
    }
    return textMap[status] || status
}
</script>

<style scoped>
.cloud-send-settings {
    width: 100%;
}

.directory-input-wrapper {
    display: flex;
    gap: 8px;
    align-items: flex-start;
}

.directory-input-wrapper .v-text-field {
    flex: 1;
}
</style>
