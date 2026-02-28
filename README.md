# PureSend

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub release](https://img.shields.io/github/v/release/z-only/puresend?include_prereleases)](https://github.com/z-only/puresend/releases)
[![GitHub stars](https://img.shields.io/github/stars/z-only/puresend?style=social)](https://github.com/z-only/puresend/stargazers)

**语言**: [中文](README.md) | [English](README_EN.md)

**跨平台文件传输应用**，基于 Tauri 2 + Vue 3 + TypeScript 构建。支持 macOS、Windows、Linux 和 Android 平台，实现设备间快速、安全的文件传输。

📚 **文档站点**: https://z-only.github.io/puresend/

## 功能特性

### 传输方式
- 🚀 **P2P 直连传输** - 局域网内设备间高速直连传输，支持自动设备发现（mDNS）
- 🌐 **Web 下载（链接分享）** - 生成下载链接/二维码，任何浏览器可直接下载文件
- 📤 **Web 上传** - 通过浏览器向应用上传文件，支持按 IP 审批

### 传输能力
- 📦 **多内容类型** - 支持文件、文件夹、剪贴板、文本、媒体、应用等 6 种内容类型
- 🔄 **断点续传** - 传输中断后可从断点恢复，无需重新传输
- ⚡ **分块传输** - 大文件自动分块，支持并行传输
- 🗜️ **动态压缩** - 基于 zstd 算法的智能压缩，自动判断是否压缩以优化传输速度

### 安全与隐私
- 🔒 **传输加密** - AES-256-GCM 端到端加密，P-256 ECDH 密钥交换
- 🔑 **PIN 保护** - Web 下载链接可设置 PIN 码访问保护
- 👁️ **隐私模式** - 可关闭传输历史记录

### 用户体验
- 📱 **跨平台** - 支持 macOS、Windows、Linux 和 Android
- 📋 **传输历史** - 记录传输历史，支持筛选和排序
- 🌍 **多语言** - 支持中文、英文，跟随系统语言
- 🎨 **主题设置** - 浅色/深色/系统主题切换
- 📐 **自定义界面** - Tab 栏布局配置、字体大小调节
- 🎯 **设备发现** - 基于 mDNS 的局域网自动设备发现

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

## CI/CD 构建

本项目使用 GitHub Actions 实现全平台自动化构建。

### 触发构建

**方式一：推送 Tag**
```bash
# 创建并推送版本标签，自动触发构建
git tag v1.0.0
git push origin v1.0.0
```

**方式二：手动触发**
1. 进入 GitHub 仓库的 **Actions** 页面
2. 选择 **Build and Release** 工作流
3. 点击 **Run workflow**，选择构建选项

### 构建产物

构建完成后，产物可在以下位置下载：

| 来源 | 说明 |
|------|------|
| **GitHub Release** | 推送 tag 后自动创建，包含所有平台安装包 |
| **Actions Artifacts** | 手动触发后可在 Actions 运行记录中下载 |

### 支持的构建平台

| 平台 | 架构 | 输出格式 |
|------|------|----------|
| macOS (Intel) | x64 | .app, .dmg |
| macOS (Apple Silicon) | arm64 | .app, .dmg |
| Windows | x64 | .msi, .exe (NSIS) |
| Linux | x64 | .deb, .AppImage, .rpm |
| Android | arm64, armv7, x86, x64 | .apk, .aab |

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 技术栈

- **前端**: Vue 3 + TypeScript + Vuetify 3 + Pinia
- **后端**: Tauri 2 (Rust)
- **构建工具**: Vite 7
