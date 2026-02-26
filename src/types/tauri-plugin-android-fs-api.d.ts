declare module 'tauri-plugin-android-fs-api' {
    type AndroidFsUri = string

    type FsPath = string | URL

    interface AndroidEntryType {
        type: 'Dir' | 'File'
        mimeType?: string
    }

    interface AndroidDirMetadata {
        type: 'Dir'
        name: string
        lastModified: number
    }

    interface AndroidFileMetadata {
        type: 'File'
        name: string
        lastModified: number
        byteLength: number
        mimeType: string
    }

    type AndroidEntryMetadata = AndroidDirMetadata | AndroidFileMetadata

    type AndroidEntryMetadataWithUri = AndroidEntryMetadata & {
        uri: AndroidFsUri
        isDir: boolean
        isFile: boolean
    }

    interface AndroidOpenDirPickerOptions {
        localOnly?: boolean
        initialLocation?: AndroidPickerInitialLocation
    }

    interface AndroidOpenFilePickerOptions {
        mimeTypes?: string[]
        multiple?: boolean
        pickerType?: 'FilePicker' | 'Gallery'
        needWritePermission?: boolean
        localOnly?: boolean
        initialLocation?: AndroidPickerInitialLocation
    }

    interface AndroidSaveFilePickerOptions {
        localOnly?: boolean
        initialLocation?: AndroidPickerInitialLocation
    }

    type AndroidPickerInitialLocation = Record<string, unknown>

    type AndroidUriPermissionState = 'Read' | 'Write' | 'ReadAndWrite' | 'ReadOrWrite'

    export class AndroidFs {
        private constructor()
        static getName(uri: AndroidFsUri | FsPath): Promise<string>
        static getByteLength(uri: AndroidFsUri | FsPath): Promise<number>
        static getType(uri: AndroidFsUri | FsPath): Promise<AndroidEntryType>
        static getMimeType(uri: AndroidFsUri | FsPath): Promise<string>
        static getMetadata(uri: AndroidFsUri | FsPath): Promise<AndroidEntryMetadata>
        static readDir(uri: AndroidFsUri): Promise<AndroidEntryMetadataWithUri[]>
        static readFile(uri: AndroidFsUri | FsPath): Promise<Uint8Array>
        static writeFile(uri: AndroidFsUri | FsPath, data: Uint8Array | ReadableStream<Uint8Array>, options?: Record<string, unknown>): Promise<void>
        static writeTextFile(uri: AndroidFsUri | FsPath, data: string, options?: Record<string, unknown>): Promise<void>
        static copyFile(srcUri: AndroidFsUri | FsPath, destUri: AndroidFsUri | FsPath, options?: Record<string, unknown>): Promise<void>
        static truncateFile(uri: AndroidFsUri): Promise<void>
        static renameFile(uri: AndroidFsUri, name: string): Promise<AndroidFsUri>
        static renameDir(uri: AndroidFsUri, name: string): Promise<AndroidFsUri>
        static removeFile(uri: AndroidFsUri): Promise<void>
        static removeDirAll(uri: AndroidFsUri): Promise<void>
        static removeEmptyDir(uri: AndroidFsUri): Promise<void>
        static createDir(baseDirUri: AndroidFsUri, relativePath: string): Promise<AndroidFsUri>
        static createNewFile(baseDirUri: AndroidFsUri, relativePath: string, mimeType?: string): Promise<AndroidFsUri>
        static getFsPath(uri: AndroidFsUri | FsPath): Promise<FsPath>
        static showOpenFilePicker(options?: AndroidOpenFilePickerOptions): Promise<AndroidFsUri[]>
        static showOpenDirPicker(options?: AndroidOpenDirPickerOptions): Promise<AndroidFsUri | null>
        static showSaveFilePicker(defaultFileName: string, mimeType: string | null, options?: AndroidSaveFilePickerOptions): Promise<AndroidFsUri | null>
        static showShareFileDialog(uris: AndroidFsUri | AndroidFsUri[]): Promise<void>
        static showViewFileDialog(uri: AndroidFsUri): Promise<void>
        static showViewDirDialog(uri: AndroidFsUri): Promise<void>
        static checkPickerUriPermission(uri: AndroidFsUri, state: AndroidUriPermissionState): Promise<boolean>
        static persistPickerUriPermission(uri: AndroidFsUri): Promise<void>
        static checkPersistedPickerUriPermission(uri: AndroidFsUri, state: AndroidUriPermissionState): Promise<boolean>
        static releasePersistedPickerUriPermission(uri: AndroidFsUri): Promise<boolean>
        static releaseAllPersistedPickerUriPermissions(): Promise<void>
    }

    export function isAndroid(): boolean
    export function getAndroidApiLevel(): Promise<number>

    export const AndroidPickerInitialLocation: Readonly<{
        readonly Any: (uri: AndroidFsUri) => AndroidPickerInitialLocation
        readonly VolumeTop: (volumeId?: string) => AndroidPickerInitialLocation
        readonly PublicDir: (baseDir: string, options?: Record<string, unknown>) => AndroidPickerInitialLocation
    }>

    export {
        AndroidFsUri,
        FsPath,
        AndroidEntryType,
        AndroidEntryMetadata,
        AndroidDirMetadata,
        AndroidFileMetadata,
        AndroidEntryMetadataWithUri,
        AndroidOpenDirPickerOptions,
        AndroidOpenFilePickerOptions,
        AndroidSaveFilePickerOptions,
        AndroidUriPermissionState,
    }
}
