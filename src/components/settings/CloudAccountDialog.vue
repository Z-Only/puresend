<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useCloudStore } from '@/stores/cloud'
import type {
    CloudAccount,
    CloudAccountInput,
    CloudType,
    WebDAVCredentials,
    AliyunOSSCredentials,
    AliyunDriveCredentials,
} from '@/types/cloud'
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
const showAccessKeySecret = ref(false)
const isTestingConnection = ref(false)
const isSaving = ref(false)
const saveError = ref('')
const connectionTestPassed = ref(false)
const testResultMessage = ref('')

// WebDAV 表单数据
const webdavForm = ref({
    serverUrl: '',
    username: '',
    password: '',
})

// 阿里云 OSS 表单数据
const ossForm = ref({
    bucket: '',
    region: '',
    accessKeyId: '',
    accessKeySecret: '',
    customDomain: '',
})

// 阿里云盘表单数据
const driveForm = ref({
    refreshToken: '',
})

const formData = ref({
    name: '',
    cloudType: 'webDAV' as CloudType,
})

const cloudTypeOptions = computed(() => [
    { title: 'WebDAV', value: 'webDAV' },
    { title: t('cloudAccount.aliyunOSS'), value: 'aliyunOSS' },
    { title: t('cloudAccount.aliyunDrive'), value: 'aliyunDrive' },
])

// 阿里云 OSS Region 选项
const ossRegionOptions = computed(() => [
    { title: '华东1（杭州）', value: 'oss-cn-hangzhou' },
    { title: '华东2（上海）', value: 'oss-cn-shanghai' },
    { title: '华北1（青岛）', value: 'oss-cn-qingdao' },
    { title: '华北2（北京）', value: 'oss-cn-beijing' },
    { title: '华北3（张家口）', value: 'oss-cn-zhangjiakou' },
    { title: '华北5（呼和浩特）', value: 'oss-cn-huhehaote' },
    { title: '华北6（乌兰察布）', value: 'oss-cn-wulanchabu' },
    { title: '华南1（深圳）', value: 'oss-cn-shenzhen' },
    { title: '华南2（广州）', value: 'oss-cn-guangzhou' },
    { title: '华南3（广州）', value: 'oss-cn-guangzhou-2' },
    { title: '西南1（成都）', value: 'oss-cn-chengdu' },
    { title: '中国香港', value: 'oss-cn-hongkong' },
    { title: '美国西部1（硅谷）', value: 'oss-us-west-1' },
    { title: '美国东部1（弗吉尼亚）', value: 'oss-us-east-1' },
    { title: '亚太东南1（新加坡）', value: 'oss-ap-southeast-1' },
    { title: '亚太东南2（悉尼）', value: 'oss-ap-southeast-2' },
    { title: '亚太东南3（吉隆坡）', value: 'oss-ap-southeast-3' },
    { title: '亚太东南5（雅加达）', value: 'oss-ap-southeast-5' },
    { title: '亚太东北1（日本）', value: 'oss-ap-northeast-1' },
    { title: '亚太南部1（孟买）', value: 'oss-ap-south-1' },
    { title: '欧洲中部1（法兰克福）', value: 'oss-eu-central-1' },
    { title: '英国（伦敦）', value: 'oss-eu-west-1' },
])

const nameRules = [
    (value: string) => {
        if (!value) return t('cloudAccount.namePlaceholder')
        if (value.trim().length === 0) return t('cloudAccount.namePlaceholder')
        return true
    },
]

// WebDAV 验证规则
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

// OSS 验证规则
const bucketRules = [
    (value: string) => {
        if (!value) return t('cloudAccount.bucketPlaceholder')
        if (value.trim().length === 0)
            return t('cloudAccount.bucketPlaceholder')
        return true
    },
]

const regionRules = [
    (value: string) => {
        if (!value) return t('cloudAccount.regionPlaceholder')
        return true
    },
]

const accessKeyIdRules = [
    (value: string) => {
        if (!value) return t('cloudAccount.accessKeyIdPlaceholder')
        if (value.trim().length === 0)
            return t('cloudAccount.accessKeyIdPlaceholder')
        return true
    },
]

const accessKeySecretRules = [
    (value: string) => {
        if (!value) return t('cloudAccount.accessKeySecretPlaceholder')
        if (value.trim().length === 0)
            return t('cloudAccount.accessKeySecretPlaceholder')
        return true
    },
]

// 阿里云盘验证规则
const refreshTokenRules = [
    (value: string) => {
        if (!value) return t('cloudAccount.refreshTokenPlaceholder')
        if (value.trim().length === 0)
            return t('cloudAccount.refreshTokenPlaceholder')
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
                // 需要从后端获取凭证信息
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
            // 根据云盘类型填充对应的表单
            if (props.account.cloudType === 'webDAV') {
                webdavForm.value.serverUrl = credentials.serverUrl || ''
                webdavForm.value.username = credentials.username || ''
                webdavForm.value.password = credentials.password || ''
            } else if (props.account.cloudType === 'aliyunOSS') {
                ossForm.value.bucket = credentials.bucket || ''
                ossForm.value.region = credentials.region || ''
                ossForm.value.accessKeyId = credentials.accessKeyId || ''
                ossForm.value.accessKeySecret =
                    credentials.accessKeySecret || ''
                ossForm.value.customDomain = credentials.customDomain || ''
            } else if (props.account.cloudType === 'aliyunDrive') {
                driveForm.value.refreshToken = credentials.refreshToken || ''
            }
        }
    } catch (error) {
        console.error('[CloudAccountDialog] 加载账号凭证失败:', error)
    }
}

function resetForm() {
    formData.value = {
        name: '',
        cloudType: 'webDAV',
    }
    webdavForm.value = {
        serverUrl: '',
        username: '',
        password: '',
    }
    ossForm.value = {
        bucket: '',
        region: '',
        accessKeyId: '',
        accessKeySecret: '',
        customDomain: '',
    }
    driveForm.value = {
        refreshToken: '',
    }
    showPassword.value = false
    showAccessKeySecret.value = false
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

// 构建凭证对象
function buildCredentials():
    | WebDAVCredentials
    | AliyunOSSCredentials
    | AliyunDriveCredentials {
    switch (formData.value.cloudType) {
        case 'webDAV':
            return {
                type: 'webDAV',
                serverUrl: webdavForm.value.serverUrl.trim(),
                username: webdavForm.value.username.trim(),
                password: webdavForm.value.password,
            }
        case 'aliyunOSS':
            return {
                type: 'aliyunOSS',
                bucket: ossForm.value.bucket.trim(),
                region: ossForm.value.region,
                accessKeyId: ossForm.value.accessKeyId.trim(),
                accessKeySecret: ossForm.value.accessKeySecret,
                customDomain: ossForm.value.customDomain.trim() || undefined,
            }
        case 'aliyunDrive':
            return {
                type: 'aliyunDrive',
                refreshToken: driveForm.value.refreshToken.trim(),
            }
        default:
            throw new Error(
                `Unsupported cloud type: ${formData.value.cloudType}`
            )
    }
}

async function handleTestConnection() {
    const isValid = await formRef.value?.validate()
    if (!isValid) return

    isTestingConnection.value = true
    saveError.value = ''
    testResultMessage.value = ''

    try {
        const credentials = buildCredentials()
        const success = await cloudStore.testConnectionWithCredentials({
            cloudType: formData.value.cloudType,
            credentials,
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
        const credentials = buildCredentials()
        const accountInput: CloudAccountInput = {
            name: formData.value.name.trim(),
            cloudType: formData.value.cloudType,
            credentials,
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

                        <!-- WebDAV 配置 -->
                        <template v-if="formData.cloudType === 'webDAV'">
                            <v-text-field
                                v-model="webdavForm.serverUrl"
                                :label="t('cloudAccount.serverUrl')"
                                :placeholder="
                                    t('cloudAccount.serverUrlPlaceholder')
                                "
                                :rules="serverUrlRules"
                                variant="outlined"
                                density="compact"
                                required
                            />

                            <v-text-field
                                v-model="webdavForm.username"
                                :label="t('cloudAccount.username')"
                                :placeholder="
                                    t('cloudAccount.usernamePlaceholder')
                                "
                                :rules="usernameRules"
                                variant="outlined"
                                density="compact"
                                required
                            />

                            <v-text-field
                                v-model="webdavForm.password"
                                :label="t('cloudAccount.password')"
                                :placeholder="
                                    t('cloudAccount.passwordPlaceholder')
                                "
                                :rules="passwordRules"
                                :type="showPassword ? 'text' : 'password'"
                                :append-inner-icon="
                                    showPassword ? mdiEyeOff : mdiEye
                                "
                                variant="outlined"
                                density="compact"
                                required
                                @click:append-inner="
                                    showPassword = !showPassword
                                "
                            />
                        </template>

                        <!-- 阿里云 OSS 配置 -->
                        <template
                            v-else-if="formData.cloudType === 'aliyunOSS'"
                        >
                            <v-text-field
                                v-model="ossForm.bucket"
                                :label="t('cloudAccount.bucket')"
                                :placeholder="
                                    t('cloudAccount.bucketPlaceholder')
                                "
                                :rules="bucketRules"
                                variant="outlined"
                                density="compact"
                                required
                            />

                            <v-select
                                v-model="ossForm.region"
                                :label="t('cloudAccount.region')"
                                :placeholder="
                                    t('cloudAccount.regionPlaceholder')
                                "
                                :items="ossRegionOptions"
                                :rules="regionRules"
                                variant="outlined"
                                density="compact"
                                required
                            />

                            <v-text-field
                                v-model="ossForm.accessKeyId"
                                :label="t('cloudAccount.accessKeyId')"
                                :placeholder="
                                    t('cloudAccount.accessKeyIdPlaceholder')
                                "
                                :rules="accessKeyIdRules"
                                variant="outlined"
                                density="compact"
                                required
                            />

                            <v-text-field
                                v-model="ossForm.accessKeySecret"
                                :label="t('cloudAccount.accessKeySecret')"
                                :placeholder="
                                    t('cloudAccount.accessKeySecretPlaceholder')
                                "
                                :rules="accessKeySecretRules"
                                :type="
                                    showAccessKeySecret ? 'text' : 'password'
                                "
                                :append-inner-icon="
                                    showAccessKeySecret ? mdiEyeOff : mdiEye
                                "
                                variant="outlined"
                                density="compact"
                                required
                                @click:append-inner="
                                    showAccessKeySecret = !showAccessKeySecret
                                "
                            />

                            <v-text-field
                                v-model="ossForm.customDomain"
                                :label="t('cloudAccount.customDomain')"
                                :placeholder="
                                    t('cloudAccount.customDomainPlaceholder')
                                "
                                :hint="t('cloudAccount.customDomainHint')"
                                variant="outlined"
                                density="compact"
                            />
                        </template>

                        <!-- 阿里云盘配置 -->
                        <template
                            v-else-if="formData.cloudType === 'aliyunDrive'"
                        >
                            <v-text-field
                                v-model="driveForm.refreshToken"
                                :label="t('cloudAccount.refreshToken')"
                                :placeholder="
                                    t('cloudAccount.refreshTokenPlaceholder')
                                "
                                :rules="refreshTokenRules"
                                :hint="t('cloudAccount.refreshTokenHint')"
                                variant="outlined"
                                density="compact"
                                required
                            />
                        </template>
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
