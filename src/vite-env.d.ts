/// <reference types="vite/client" />

declare module '*.vue' {
    import type { DefineComponent } from 'vue'
    const component: DefineComponent<object, object, unknown>
    export default component
}

declare const __APP_VERSION__: string

declare module 'tauri-plugin-android-fs-api' {
    export function showOpenDirPicker(): Promise<string | null>
    export function readDir(
        uri: string
    ): Promise<Array<{ uri: string; isDir: boolean }>>
    export function getName(uri: string): Promise<string>
    export function getByteLength(uri: string): Promise<number>
    export function getMimeType(uri: string): Promise<string>
}
