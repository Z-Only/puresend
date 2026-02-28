# PureSend

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub release](https://img.shields.io/github/v/release/z-only/puresend?include_prereleases)](https://github.com/z-only/puresend/releases)
[![GitHub stars](https://img.shields.io/github/stars/z-only/puresend?style=social)](https://github.com/z-only/puresend/stargazers)

**Language**: [中文](README.md) | [English](README_EN.md)

**Cross-platform file transfer application** built with Tauri 2 + Vue 3 + TypeScript. Supports macOS, Windows, Linux, and Android platforms for fast and secure file transfers between devices.

📚 **Documentation Site**: https://z-only.github.io/puresend/

## Features

### Transfer Modes
- 🚀 **P2P Direct Transfer** - High-speed direct transfer between devices on the same LAN with automatic device discovery (mDNS)
- 🌐 **Web Download (Link Sharing)** - Generate download links/QR codes for direct browser downloads
- 📤 **Web Upload** - Upload files to the app via browser with per-IP approval

### Transfer Capabilities
- 📦 **Multiple Content Types** - Supports files, folders, clipboard, text, media, and apps (6 content types)
- 🔄 **Resume Transfer** - Resume interrupted transfers from the breakpoint
- ⚡ **Chunked Transfer** - Automatic chunking for large files with parallel transfer
- 🗜️ **Dynamic Compression** - Smart compression based on zstd algorithm, automatically determines whether to compress

### Security & Privacy
- 🔒 **Transfer Encryption** - AES-256-GCM end-to-end encryption with P-256 ECDH key exchange
- 🔑 **PIN Protection** - Protect Web download links with PIN code access
- 👁️ **Privacy Mode** - Option to disable transfer history recording

### User Experience
- 📱 **Cross-platform** - Supports macOS, Windows, Linux, and Android
- 📋 **Transfer History** - Records transfer history with filtering and sorting
- 🌍 **Multi-language** - Supports Chinese and English, follows system language
- 🎨 **Theme Settings** - Light/Dark/System theme switching
- 📐 **Customizable UI** - Tab bar layout configuration, font size adjustment
- 🎯 **Device Discovery** - Automatic LAN device discovery based on mDNS

## Supported Platforms

| Platform | Build Command              | Output Format         | Minimum Version      |
| -------- | -------------------------- | --------------------- | -------------------- |
| macOS    | `pnpm tauri build`         | .app, .dmg            | macOS 10.13          |
| Windows  | `pnpm tauri build`         | .msi, .nsis           | Windows 7            |
| Linux    | `pnpm tauri build`         | .deb, .appimage, .rpm | -                    |
| Android  | `pnpm tauri android build` | .apk, .aab            | API 24 (Android 7.0) |

## Development Environment

### Desktop Development

```bash
# Install dependencies
pnpm install

# Start development server
pnpm tauri dev

# Build desktop application
pnpm tauri build
```

### Android Development

#### Requirements

1. **Android Studio** - Install Android SDK and NDK
2. **JDK 17+** - Java Development Kit
3. **Rust Android Targets** - Install by running:
    ```bash
    rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
    ```

#### Environment Variables Configuration

Add to `~/.zshrc` or `~/.bashrc`:

```bash
export ANDROID_HOME=$HOME/Library/Android/sdk
export NDK_HOME=$ANDROID_HOME/ndk/<ndk-version>
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin
export PATH=$PATH:$ANDROID_HOME/platform-tools
```

#### Android Build Commands

```bash
# Initialize Android project (first time only)
pnpm tauri android init

# Development mode
pnpm tauri android dev

# Build Debug APK
pnpm tauri android build

# Build Release APK/AAB
pnpm tauri android build --release
```

## CI/CD Build

This project uses GitHub Actions for full-platform automated builds.

### Triggering Builds

**Method 1: Push Tag**

```bash
# Create and push version tag to automatically trigger build
git tag v1.0.0
git push origin v1.0.0
```

**Method 2: Manual Trigger**

1. Go to the **Actions** page of the GitHub repository
2. Select the **Build and Release** workflow
3. Click **Run workflow** and choose build options

### Build Artifacts

After build completion, artifacts can be downloaded from:

| Source                | Description                                                               |
| --------------------- | ------------------------------------------------------------------------- |
| **GitHub Release**    | Automatically created after pushing tag, contains all platform installers |
| **Actions Artifacts** | Can be downloaded from Actions run records when manually triggered        |

### Supported Build Platforms

| Platform              | Architecture           | Output Format         |
| --------------------- | ---------------------- | --------------------- |
| macOS (Intel)         | x64                    | .app, .dmg            |
| macOS (Apple Silicon) | arm64                  | .app, .dmg            |
| Windows               | x64                    | .msi, .exe (NSIS)     |
| Linux                 | x64                    | .deb, .AppImage, .rpm |
| Android               | arm64, armv7, x86, x64 | .apk, .aab            |

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Tech Stack

- **Frontend**: Vue 3 + TypeScript + Vuetify 3 + Pinia
- **Backend**: Tauri 2 (Rust)
- **Build Tool**: Vite 7
