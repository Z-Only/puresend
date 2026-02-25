# 开发环境搭建

本指南将帮助您搭建 PureSend 的开发环境。

## 系统要求

### 必需软件

- **Node.js** >= 18.x
- **pnpm** >= 8.x
- **Rust** >= 1.70
- **系统依赖**: 参考 [Tauri 官方文档](https://tauri.app/start/prerequisites/)

### 平台特定要求

#### macOS

```bash
# 安装 Xcode 命令行工具
xcode-select --install

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### Windows

1. 安装 [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
2. 安装 [Rust](https://www.rust-lang.org/tools/install)

#### Linux

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
    build-essential \
    curl \
    wget \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## 快速开始

### 1. 克隆仓库

```bash
git clone https://github.com/Z-Only/puresend.git
cd puresend
```

### 2. 安装依赖

```bash
# 安装 pnpm（如果尚未安装）
npm install -g pnpm

# 安装项目依赖
pnpm install
```

### 3. 启动开发服务器

```bash
pnpm tauri dev
```

这将同时启动：
- Vite 开发服务器（前端热更新）
- Tauri 应用窗口（Rust 后端）

## 项目结构

```
puresend/
├── src/                    # Vue 前端代码
│   ├── components/         # Vue 组件
│   ├── composables/        # 组合式函数
│   ├── services/           # 服务层
│   ├── stores/             # Pinia 状态管理
│   ├── types/              # TypeScript 类型定义
│   └── views/              # 页面视图
├── src-tauri/              # Tauri/Rust 后端
│   ├── src/                # Rust 源代码
│   ├── Cargo.toml          # Rust 依赖
│   └── tauri.conf.json     # Tauri 配置
├── docs/                   # VitePress 文档
└── package.json            # Node.js 依赖
```

## 技术栈

- **前端**: Vue 3 + TypeScript + Vuetify 3 + Pinia
- **后端**: Tauri 2 (Rust)
- **构建工具**: Vite 7
- **文档**: VitePress

## 常用命令

| 命令                | 说明                     |
| ------------------- | ------------------------ |
| `pnpm tauri dev`    | 启动开发服务器           |
| `pnpm tauri build`  | 构建生产版本             |
| `pnpm lint`         | 运行代码检查             |
| `pnpm format`       | 格式化代码               |
| `pnpm docs:dev`     | 启动文档开发服务器       |
| `pnpm docs:build`   | 构建文档站点             |

## Android 开发

### 环境要求

1. **Android Studio** - 安装 Android SDK 和 NDK
2. **JDK 17+** - Java 开发工具包
3. **Rust Android 目标**:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

### 环境变量

在 `~/.zshrc` 或 `~/.bashrc` 中添加：

```bash
export ANDROID_HOME=$HOME/Library/Android/sdk
export NDK_HOME=$ANDROID_HOME/ndk/<ndk-version>
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin
export PATH=$PATH:$ANDROID_HOME/platform-tools
```

### 构建命令

```bash
# 初始化 Android 项目（首次）
pnpm tauri android init

# 开发模式
pnpm tauri android dev

# 构建 APK
pnpm tauri android build

# 构建 Release 版本
pnpm tauri android build --release
```

## IDE 推荐

- [VS Code](https://code.visualstudio.com/)
- 扩展：
  - [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar)
  - [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
  - [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 下一步

- 了解 [文件传输功能](/features/file-transfer)
- 阅读 [贡献指南](https://github.com/Z-Only/puresend/blob/main/CONTRIBUTING.md)
