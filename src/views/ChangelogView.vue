<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { marked } from 'marked'
import { mdiArrowLeft } from '@mdi/js'
import { getCurrentLocale } from '@/i18n'
import changelogZh from '../../CHANGELOG.md?raw'
import changelogEn from '../../CHANGELOG_EN.md?raw'

const router = useRouter()
const { t } = useI18n()

// 根据当前语言选择对应的 changelog 文件
const changelogContent = computed(() => {
    const locale = getCurrentLocale()
    return locale === 'zh-CN' ? changelogZh : changelogEn
})

/**
 * 过滤 changelog 内容，只保留版本号和功能相关内容
 * 移除文件头的说明文字和底部的链接引用
 */
const filteredChangelog = computed(() => {
    const content = changelogContent.value
    const lines = content.split('\n')
    const filteredLines: string[] = []
    let inVersionSection = false
    let skipHeader = true

    for (const line of lines) {
        // 跳过文件开头的说明文字（直到遇到第一个版本号标题）
        if (skipHeader) {
            // 检测版本号标题：## [version] 或 ## [Unreleased]
            if (/^## \[.*\]/.test(line)) {
                skipHeader = false
                inVersionSection = true
            } else {
                continue
            }
        }

        // 跳过底部的链接引用（以 --- 或 [链接]: 开头的部分）
        if (line.trim() === '---') {
            inVersionSection = false
            continue
        }
        if (/^\[.*\]:/.test(line)) {
            continue
        }

        if (inVersionSection) {
            filteredLines.push(line)
        }
    }

    return filteredLines.join('\n')
})

const renderedChangelog = computed(() => {
    return marked(filteredChangelog.value) as string
})

function goBack() {
    router.back()
}
</script>

<template>
    <v-container class="changelog-view" max-width="800">
        <div class="d-flex align-center mb-4">
            <v-btn variant="text" :icon="mdiArrowLeft" @click="goBack" />
            <h1 class="text-h4 ml-2">
                {{ t('settings.about.changelog') }}
            </h1>
        </div>

        <v-card>
            <v-card-text>
                <!-- eslint-disable-next-line vue/no-v-html -->
                <div class="changelog-content" v-html="renderedChangelog" />
            </v-card-text>
        </v-card>
    </v-container>
</template>

<style scoped>
.changelog-content :deep(h2) {
    font-size: 1.25rem;
    font-weight: 600;
    margin-top: 1.5rem;
    margin-bottom: 0.75rem;
}

.changelog-content :deep(h2:first-child) {
    margin-top: 0;
}

.changelog-content :deep(h3) {
    font-size: 1rem;
    font-weight: 600;
    margin-top: 1rem;
    margin-bottom: 0.5rem;
}

.changelog-content :deep(ul) {
    padding-left: 1.5rem;
    margin-bottom: 0.75rem;
}

.changelog-content :deep(li) {
    margin-bottom: 0.25rem;
}

.changelog-content :deep(code) {
    background-color: rgba(var(--v-theme-on-surface), 0.08);
    padding: 0.125rem 0.375rem;
    border-radius: 4px;
    font-size: 0.875em;
}

.changelog-content :deep(a) {
    color: rgb(var(--v-theme-primary));
    text-decoration: none;
}

.changelog-content :deep(a:hover) {
    text-decoration: underline;
}
</style>
