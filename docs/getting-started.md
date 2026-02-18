# 快速开始

本指南将帮助您快速上手 PureSend。

## 系统要求

### 桌面端

| 平台    | 最低版本      |
| ------- | ------------- |
| macOS   | macOS 10.13   |
| Windows | Windows 7     |
| Linux   | 主流发行版    |

### 移动端

| 平台    | 最低版本          |
| ------- | ----------------- |
| Android | API 24 (Android 7.0) |

## 安装

### 下载应用

前往 [GitHub Releases](https://github.com/Z-Only/puresend/releases) 页面下载适合您平台的安装包：

- **macOS**: 下载 `.dmg` 文件
- **Windows**: 下载 `.msi` 或 `.exe` 安装程序
- **Linux**: 下载 `.deb`、`.rpm` 或 `.AppImage` 文件
- **Android**: 下载 `.apk` 文件

### 安装步骤

#### macOS

1. 打开下载的 `.dmg` 文件
2. 将 PureSend 拖拽到 Applications 文件夹
3. 首次运行可能需要在「系统偏好设置 > 安全性与隐私」中允许运行

#### Windows

1. 运行下载的 `.msi` 或 `.exe` 安装程序
2. 按照安装向导完成安装

#### Linux

**Debian/Ubuntu:**

```bash
sudo dpkg -i puresend_*.deb
```

**Fedora/RHEL:**

```bash
sudo rpm -i puresend_*.rpm
```

**AppImage:**

```bash
chmod +x puresend_*.AppImage
./puresend_*.AppImage
```

## 基本使用

### 发送文件

1. 打开 PureSend 应用
2. 点击「发送」标签
3. 选择要发送的文件或文件夹
4. 等待接收方扫描二维码或输入配对码

### 接收文件

1. 打开 PureSend 应用
2. 点击「接收」标签
3. 扫描发送方的二维码，或输入配对码
4. 等待连接建立
5. 选择保存位置，开始接收

## 下一步

- 了解更多 [文件传输功能](/features/file-transfer)
- 查看 [开发指南](/development/setup)
