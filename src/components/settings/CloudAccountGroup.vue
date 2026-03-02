<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useCloudStore } from '@/stores/cloud'
import type { CloudAccount } from '@/types/cloud'
import CloudAccountDialog from './CloudAccountDialog.vue'
import SettingsGroup from './SettingsGroup.vue'
import {
    mdiPlus,
    mdiCloudOutline,
    mdiConnection,
    mdiPencil,
    mdiDelete,
} from '@mdi/js'

const { t } = useI18n()
const cloudStore = useCloudStore()

const dialogVisible = ref(false)
const editingAccount = ref<CloudAccount | null>(null)
const deleteConfirmVisible = ref(false)
const accountToDelete = ref<CloudAccount | null>(null)
const deletingAccountIds = ref<Set<string>>(new Set())
const testingAccountIds = ref<Set<string>>(new Set())
const testResults = ref<Map<string, { success: boolean; message: string }>>(
    new Map()
)

const accounts = computed(() => cloudStore.accounts)
const loading = computed(() => cloudStore.loading)

const cloudTypeLabels: Record<string, string> = {
    webDAV: 'WebDAV',
    aliyunOSS: t('cloudAccount.aliyunOSS'),
    aliyunDrive: t('cloudAccount.aliyunDrive'),
}

function getStatusColor(status: string): string {
    switch (status) {
        case 'connected':
            return 'success'
        case 'disconnected':
            return 'grey'
        case 'invalid':
            return 'error'
        default:
            return 'grey'
    }
}

function getStatusLabel(status: string): string {
    return t(`cloudAccount.status.${status}`)
}

function openAddDialog() {
    editingAccount.value = null
    dialogVisible.value = true
}

function openEditDialog(account: CloudAccount) {
    editingAccount.value = account
    dialogVisible.value = true
}

function closeDialog() {
    dialogVisible.value = false
    editingAccount.value = null
}

function handleDialogClose() {
    closeDialog()
}

async function handleDialogSuccess() {
    closeDialog()
    await cloudStore.loadAccounts()
}

function openDeleteConfirm(account: CloudAccount) {
    accountToDelete.value = account
    deleteConfirmVisible.value = true
}

function closeDeleteConfirm() {
    deleteConfirmVisible.value = false
    accountToDelete.value = null
}

async function handleDeleteConfirm() {
    if (!accountToDelete.value) return

    const accountId = accountToDelete.value.id
    deletingAccountIds.value.add(accountId)

    try {
        await cloudStore.deleteAccount(accountId)
    } catch (error) {
        console.error('[CloudAccountGroup] 删除账号失败:', error)
    } finally {
        deletingAccountIds.value.delete(accountId)
        closeDeleteConfirm()
    }
}

const emit = defineEmits<{
    testResult: [
        result: { accountId: string; success: boolean; message: string },
    ]
}>()

async function handleTestConnection(account: CloudAccount) {
    if (testingAccountIds.value.has(account.id)) return

    testingAccountIds.value.add(account.id)
    testResults.value.delete(account.id)

    try {
        const success = await cloudStore.testConnection(account.id)
        const result = {
            accountId: account.id,
            success,
            message: success
                ? t('cloudAccount.testSuccess')
                : t('cloudAccount.testFailed'),
        }
        testResults.value.set(account.id, result)
        emit('testResult', result)
    } catch (error) {
        console.error('[CloudAccountGroup] 测试连接失败:', error)
        const result = {
            accountId: account.id,
            success: false,
            message: t('cloudAccount.testFailed'),
        }
        testResults.value.set(account.id, result)
        emit('testResult', result)
    } finally {
        testingAccountIds.value.delete(account.id)
    }
}

onMounted(() => {
    cloudStore.loadAccounts()
})
</script>

<template>
    <SettingsGroup :title="t('cloudAccount.title')">
        <div class="d-flex flex-column ga-4">
            <!-- 添加账号按钮 -->
            <div class="d-flex justify-end">
                <v-btn
                    color="primary"
                    variant="tonal"
                    :prepend-icon="mdiPlus"
                    @click="openAddDialog"
                >
                    {{ t('cloudAccount.add') }}
                </v-btn>
            </div>

            <!-- 账号列表 -->
            <div v-if="!loading && accounts.length > 0" class="account-list">
                <v-card
                    v-for="account in accounts"
                    :key="account.id"
                    variant="outlined"
                    class="account-card"
                >
                    <div class="account-card-content">
                        <div class="account-info">
                            <div class="d-flex align-center ga-2">
                                <div class="text-h6">{{ account.name }}</div>
                                <v-chip
                                    :color="getStatusColor(account.status)"
                                    size="small"
                                    variant="flat"
                                >
                                    {{ getStatusLabel(account.status) }}
                                </v-chip>
                            </div>
                            <div class="text-body-2 text-grey mt-1">
                                {{
                                    cloudTypeLabels[account.cloudType] ||
                                    account.cloudType
                                }}
                            </div>
                            <div
                                v-if="account.defaultDirectory"
                                class="text-body-2 text-grey mt-1"
                            >
                                {{ t('cloudAccount.defaultDirectory') }}:
                                {{ account.defaultDirectory }}
                            </div>
                        </div>

                        <div class="account-actions">
                            <v-btn
                                :color="
                                    testingAccountIds.has(account.id)
                                        ? 'grey'
                                        : 'primary'
                                "
                                :disabled="testingAccountIds.has(account.id)"
                                :loading="testingAccountIds.has(account.id)"
                                size="small"
                                variant="text"
                                @click="handleTestConnection(account)"
                            >
                                <v-icon :icon="mdiConnection" start />
                                {{ t('cloudAccount.testConnection') }}
                            </v-btn>
                            <v-btn
                                color="primary"
                                size="small"
                                variant="text"
                                @click="openEditDialog(account)"
                            >
                                <v-icon :icon="mdiPencil" start />
                                {{ t('cloudAccount.edit') }}
                            </v-btn>
                            <v-btn
                                :color="
                                    deletingAccountIds.has(account.id)
                                        ? 'grey'
                                        : 'error'
                                "
                                :disabled="deletingAccountIds.has(account.id)"
                                :loading="deletingAccountIds.has(account.id)"
                                size="small"
                                variant="text"
                                @click="openDeleteConfirm(account)"
                            >
                                <v-icon :icon="mdiDelete" start />
                                {{ t('cloudAccount.delete') }}
                            </v-btn>
                        </div>
                    </div>
                </v-card>
            </div>

            <!-- 空状态 -->
            <div v-else-if="!loading" class="empty-state">
                <v-icon
                    :icon="mdiCloudOutline"
                    size="64"
                    color="grey-lighten-1"
                />
                <div class="text-h6 text-grey mt-4">
                    {{ t('cloudAccount.noAccounts') }}
                </div>
                <div class="text-body-2 text-grey mt-2">
                    {{ t('cloudAccount.noAccountsHint') }}
                </div>
            </div>

            <!-- 加载状态 -->
            <div v-else class="d-flex justify-center py-8">
                <v-progress-circular indeterminate color="primary" />
            </div>
        </div>

        <!-- 新增/编辑对话框 -->
        <CloudAccountDialog
            v-model="dialogVisible"
            :account="editingAccount"
            @close="handleDialogClose"
            @success="handleDialogSuccess"
        />

        <!-- 删除确认对话框 -->
        <v-dialog v-model="deleteConfirmVisible" max-width="400">
            <v-card>
                <v-card-title class="text-h6">
                    {{ t('cloudAccount.deleteConfirm.title') }}
                </v-card-title>
                <v-card-text>
                    {{
                        t('cloudAccount.deleteConfirm.message', {
                            name: accountToDelete?.name,
                        })
                    }}
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn variant="text" @click="closeDeleteConfirm">
                        {{ t('common.cancel') }}
                    </v-btn>
                    <v-btn
                        color="error"
                        variant="elevated"
                        :loading="
                            deletingAccountIds.has(accountToDelete?.id || '')
                        "
                        @click="handleDeleteConfirm"
                    >
                        {{ t('common.confirm') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>
    </SettingsGroup>
</template>

<style scoped>
.account-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.account-card {
    border-radius: 8px;
}

.account-card-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px;
    gap: 16px;
}

.account-info {
    flex: 1;
    min-width: 0;
}

.account-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
}

.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
}

@media (max-width: 600px) {
    .account-card-content {
        flex-direction: column;
        align-items: flex-start;
    }

    .account-actions {
        width: 100%;
        justify-content: flex-end;
        flex-wrap: wrap;
    }
}
</style>
