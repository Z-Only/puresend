import js from '@eslint/js'
import globals from 'globals'
import tseslint from 'typescript-eslint'
import pluginVue from 'eslint-plugin-vue'
import { defineConfig } from 'eslint/config'
import autoImportConfig from './.eslintrc-auto-import.json'

export default defineConfig([
    // 忽略编译产物
    {
        ignores: [
            'dist/**',
            'node_modules/**',
            'src-tauri/target/**',
            '*.d.ts',
            'auto-imports.d.ts',
            'components.d.ts',
        ],
    },
    {
        files: ['**/*.{js,mjs,cjs,ts,mts,cts,vue}'],
        extends: [js.configs.recommended],
        languageOptions: {
            globals: {
                ...globals.browser,
                ...globals.node,
                __APP_VERSION__: 'readonly',
            },
        },
    },
    ...tseslint.configs.recommended,
    ...pluginVue.configs['flat/essential'],
    {
        files: ['**/*.vue'],
        languageOptions: { parserOptions: { parser: tseslint.parser } },
    },
    autoImportConfig,
    // 禁用一些严格规则
    {
        files: ['**/*.ts', '**/*.vue'],
        rules: {
            '@typescript-eslint/no-explicit-any': 'off',
            '@typescript-eslint/no-empty-object-type': 'off',
        },
    },
])
