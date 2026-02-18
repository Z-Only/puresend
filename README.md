# PureSend

跨平台文件传输应用，基于 Tauri 2 + Vue 3 + TypeScript 构建。

📚 **文档站点**: https://z-only.github.io/puresend/

## 支持的平台

| 平台    | 构建命令              | 输出格式              | 最低版本             |
| ------- | --------------------- | --------------------- | -------------------- |
| macOS   | `pnpm tauri build`    | .app, .dmg            | macOS 10.13          |
| Windows | `pnpm tauri build`    | .msi, .nsis           | Windows 7            |
| Linux   | `pnpm tauri build`    | .deb, .appimage, .rpm | -                    |
| Android | `pnpm tauri android build` | .apk, .aab       | API 24 (Android 7.0) |

## 开发环境

### 桌面端开发

```bash
# 安装依赖
pnpm install

# 启动开发服务器
pnpm tauri dev

# 构建桌面应用
pnpm tauri build
```

### Android 开发

#### 环境要求

1. **Android Studio** - 安装 Android SDK 和 NDK
2. **JDK 17+** - Java 开发工具包
3. **Rust Android 目标** - 运行以下命令安装：
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
   ```

#### 环境变量配置

在 `~/.zshrc` 或 `~/.bashrc` 中添加：

```bash
export ANDROID_HOME=$HOME/Library/Android/sdk
export NDK_HOME=$ANDROID_HOME/ndk/<ndk-version>
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin
export PATH=$PATH:$ANDROID_HOME/platform-tools
```

#### Android 构建命令

```bash
# 初始化 Android 项目（首次）
pnpm tauri android init

# 开发模式
pnpm tauri android dev

# 构建 Debug APK
pnpm tauri android build

# 构建 Release APK/AAB
pnpm tauri android build --release
```

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 技术栈

- **前端**: Vue 3 + TypeScript + Vuetify 3 + Pinia
- **后端**: Tauri 2 (Rust)
- **构建工具**: Vite 6
