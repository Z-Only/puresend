<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useCloudStore } from '@/stores/cloud'
import type { CloudAccount, CloudAccountInput, CloudType } from '@/types/cloud'
import { mdiEye, mdiEyeOff, mdiConnection } from '@mdi/js'

interface Props {
    modelValue: boolean
    account?: CloudAccount | null
}

interface Emits {
    (e: 'update:modelValue', value: boolean): void
    (e: 'close'): void
    (e: 'success'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const { t } = useI18n()
const cloudStore = useCloudStore()

const formRef = ref()
const valid = ref(false)
const isEditMode = computed(() => !!props.account)
const dialogTitle = computed(() =>
    isEditMode.value ? t('cloudAccount.edit') : t('cloudAccount.add')
)

const showPassword = ref(false)
const isTestingConnection = ref(false)
const isSaving = ref(false)
const saveError = ref('')
const connectionTestPassed = ref(false)
const testResultMessage = ref('')

const formData = ref({
    name: '',
    cloudType: 'webDAV' as CloudType,
    serverUrl: '',
    username: '',
    password: '',
})

const cloudTypeOptions = computed(() => [
    { title: 'WebDAV', value: 'webDAV' },
    { title: 'OSS', value: 'oss' },
    { title: t('cloudAccount.type'), value: 'netdisk' },
])

const nameRules = [
    (value: string) => {
        if (!value) return t('cloudAccount.namePlaceholder')
        if (value.trim().length === 0) return t('cloudAccount.namePlaceholder')
        return true
    },
]

const serverUrlRules = [
    (value: string) => {
        if (!value) return t('cloudAccount.serverUrlPlaceholder')
        if (value.trim().length === 0)
            return t('cloudAccount.serverUrlPlaceholder')
        try {
            new URL(value)
        } catch {
            return t('cloudAccount.serverUrlPlaceholder')
        }
        return true
    },
]

const usernameRules = [
    (value: string) => {
        if (!value) return t('cloudAccount.usernamePlaceholder')
        if (value.trim().length === 0)
            return t('cloudAccount.usernamePlaceholder')
        return true
    },
]

const passwordRules = [
    (value: string) => {
        if (!value) return t('cloudAccount.passwordPlaceholder')
        if (value.trim().length === 0)
            return t('cloudAccount.passwordPlaceholder')
        return true
    },
]

watch(
    () => props.modelValue,
    (newValue) => {
        if (newValue) {
            resetForm()
            if (props.account) {
                // 编辑模式：填充现有账号信息
                formData.value.name = props.account.name
                formData.value.cloudType = props.account.cloudType
                // 需要从后端获取凭证信息来填充 serverUrl、username、password
                loadAccountCredentials()
            }
        }
    }
)

async function loadAccountCredentials(): Promise<void> {
    if (!props.account) return
    try {
        const credentials = await cloudStore.getAccountCredentials(
            props.account.id
        )
        if (credentials) {
            formData.value.serverUrl = credentials.serverUrl
            formData.value.username = credentials.username
            formData.value.password = credentials.password
        }
    } catch (error) {
        console.error('[CloudAccountDialog] 加载账号凭证失败:', error)
    }
}

function resetForm() {
    formData.value = {
        name: '',
        cloudType: 'webDAV',
        serverUrl: '',
        username: '',
        password: '',
    }
    showPassword.value = false
    isTestingConnection.value = false
    isSaving.value = false
    saveError.value = ''
    valid.value = false
    connectionTestPassed.value = false
    testResultMessage.value = ''
}

function closeDialog() {
    emit('update:modelValue', false)
    emit('close')
}

async function handleTestConnection() {
    const isValid = await formRef.value?.validate()
    if (!isValid) return

    isTestingConnection.value = true
    saveError.value = ''
    testResultMessage.value = ''

    try {
        const success = await cloudStore.testConnectionWithCredentials({
            cloudType: formData.value.cloudType,
            credentials: {
                serverUrl: formData.value.serverUrl,
                username: formData.value.username,
                password: formData.value.password,
            },
        })

        connectionTestPassed.value = success
        testResultMessage.value = success
            ? t('cloudAccount.testSuccess')
            : t('cloudAccount.testFailed')
    } catch (error) {
        console.error('[CloudAccountDialog] 测试连接失败:', error)
        connectionTestPassed.value = false
        testResultMessage.value = t('cloudAccount.testFailed')
    } finally {
        isTestingConnection.value = false
    }
}

async function handleSave() {
    const isValid = await formRef.value?.validate()
    if (!isValid) return

    isSaving.value = true
    saveError.value = ''

    try {
        const accountInput: CloudAccountInput = {
            name: formData.value.name.trim(),
            cloudType: formData.value.cloudType,
            credentials: {
                serverUrl: formData.value.serverUrl.trim(),
                username: formData.value.username.trim(),
                password: formData.value.password,
            },
            defaultDirectory: '/',
            // 如果测试连接通过，设置初始状态为 connected
            initialStatus: connectionTestPassed.value ? 'connected' : undefined,
        }

        if (isEditMode.value && props.account) {
            await cloudStore.updateAccount(props.account.id, accountInput)
        } else {
            await cloudStore.addAccount(accountInput)
        }

        emit('success')
        closeDialog()
    } catch (error) {
        console.error('[CloudAccountDialog] 保存账号失败:', error)
        saveError.value = t('cloudAccount.saveError')
    } finally {
        isSaving.value = false
    }
}

function handleCancel() {
    closeDialog()
}
</script>

<template>
    <v-dialog
        :model-value="modelValue"
        max-width="600"
        @update:model-value="closeDialog"
    >
        <v-card>
            <v-card-title class="text-h6">
                {{ dialogTitle }}
            </v-card-title>

            <v-card-text>
                <v-form ref="formRef" v-model="valid">
                    <div class="d-flex flex-column ga-4">
                        <!-- 账号名称 -->
                        <v-text-field
                            v-model="formData.name"
                            :label="t('cloudAccount.name')"
                            :placeholder="t('cloudAccount.namePlaceholder')"
                            :rules="nameRules"
                            variant="outlined"
                            density="compact"
                            required
                        />

                        <!-- 云盘类型 -->
                        <v-select
                            v-model="formData.cloudType"
                            :label="t('cloudAccount.type')"
                            :items="cloudTypeOptions"
                            variant="outlined"
                            density="compact"
                            required
                        />

                        <!-- 服务器地址 -->
                        <v-text-field
                            v-model="formData.serverUrl"
                            :label="t('cloudAccount.serverUrl')"
                            :placeholder="
                                t('cloudAccount.serverUrlPlaceholder')
                            "
                            :rules="serverUrlRules"
                            variant="outlined"
                            density="compact"
                            required
                        />

                        <!-- 用户名 -->
                        <v-text-field
                            v-model="formData.username"
                            :label="t('cloudAccount.username')"
                            :placeholder="t('cloudAccount.usernamePlaceholder')"
                            :rules="usernameRules"
                            variant="outlined"
                            density="compact"
                            required
                        />

                        <!-- 密码 -->
                        <v-text-field
                            v-model="formData.password"
                            :label="t('cloudAccount.password')"
                            :placeholder="t('cloudAccount.passwordPlaceholder')"
                            :rules="passwordRules"
                            :type="showPassword ? 'text' : 'password'"
                            :append-inner-icon="
                                showPassword ? mdiEyeOff : mdiEye
                            "
                            variant="outlined"
                            density="compact"
                            required
                            @click:append-inner="showPassword = !showPassword"
                        />
                    </div>
                </v-form>

                <!-- 测试连接结果提示 -->
                <v-alert
                    v-if="testResultMessage"
                    :type="connectionTestPassed ? 'success' : 'error'"
                    variant="tonal"
                    density="compact"
                    class="mt-4"
                >
                    {{ testResultMessage }}
                </v-alert>

                <!-- 保存错误提示 -->
                <v-alert
                    v-if="saveError"
                    type="error"
                    variant="tonal"
                    density="compact"
                    class="mt-4"
                >
                    {{ saveError }}
                </v-alert>
            </v-card-text>

            <v-card-actions>
                <v-spacer />
                <v-btn variant="text" @click="handleCancel">
                    {{ t('common.cancel') }}
                </v-btn>
                <v-btn
                    color="primary"
                    variant="tonal"
                    :disabled="isTestingConnection || isSaving"
                    :loading="isTestingConnection"
                    @click="handleTestConnection"
                >
                    <v-icon :icon="mdiConnection" start />
                    {{ t('cloudAccount.testConnection') }}
                </v-btn>
                <v-btn
                    color="primary"
                    variant="elevated"
                    :disabled="!valid || isTestingConnection || isSaving"
                    :loading="isSaving"
                    @click="handleSave"
                >
                    {{ t('common.confirm') }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<style scoped>
.v-card {
    border-radius: 12px;
}
</style>
