# Development Setup

This guide will help you set up the PureSend development environment.

## System Requirements

### Required Software

- **Node.js** >= 18.x
- **pnpm** >= 8.x
- **Rust** >= 1.70
- **System Dependencies**: Refer to [Tauri Official Documentation](https://tauri.app/start/prerequisites/)

### Platform-Specific Requirements

#### macOS

```bash
# Install Xcode command line tools
xcode-select --install

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### Windows

1. Install [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
2. Install [Rust](https://www.rust-lang.org/tools/install)

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

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/Z-Only/puresend.git
cd puresend
```

### 2. Install Dependencies

```bash
# Install pnpm (if not already installed)
npm install -g pnpm

# Install project dependencies
pnpm install
```

### 3. Start Development Server

```bash
pnpm tauri dev
```

This will start:
- Vite development server (frontend hot reload)
- Tauri app window (Rust backend)

## Project Structure

```
puresend/
├── src/                    # Vue frontend code
│   ├── components/         # Vue components
│   ├── composables/        # Composables
│   ├── services/           # Service layer
│   ├── stores/             # Pinia state management
│   ├── types/              # TypeScript type definitions
│   └── views/              # Page views
├── src-tauri/              # Tauri/Rust backend
│   ├── src/                # Rust source code
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # Tauri configuration
├── docs/                   # VitePress documentation
└── package.json            # Node.js dependencies
```

## Tech Stack

- **Frontend**: Vue 3 + TypeScript + Vuetify 3 + Pinia
- **Backend**: Tauri 2 (Rust)
- **Build Tool**: Vite 7
- **Documentation**: VitePress

## Common Commands

| Command                | Description                     |
| ------------------- | ------------------------ |
| `pnpm tauri dev`    | Start development server           |
| `pnpm tauri build`  | Build production version             |
| `pnpm lint`         | Run code check             |
| `pnpm format`       | Format code               |
| `pnpm docs:dev`     | Start documentation development server       |
| `pnpm docs:build`   | Build documentation site             |

## Android Development

### Environment Requirements

1. **Android Studio** - Install Android SDK and NDK
2. **JDK 17+** - Java Development Kit
3. **Rust Android Targets**:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

### Environment Variables

Add to `~/.zshrc` or `~/.bashrc`:

```bash
export ANDROID_HOME=$HOME/Library/Android/sdk
export NDK_HOME=$ANDROID_HOME/ndk/<ndk-version>
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin
export PATH=$PATH:$ANDROID_HOME/platform-tools
```

### Build Commands

```bash
# Initialize Android project (first time)
pnpm tauri android init

# Development mode
pnpm tauri android dev

# Build APK
pnpm tauri android build

# Build Release version
pnpm tauri android build --release
```

## Recommended IDE

- [VS Code](https://code.visualstudio.com/)
- Extensions:
  - [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar)
  - [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
  - [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Next Steps

- Learn about [File Transfer Features](/features/file-transfer)
- Read the [Contributing Guide](https://github.com/Z-Only/puresend/blob/main/CONTRIBUTING.md)
