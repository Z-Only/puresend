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

### P2P 直连发送

1. 打开 PureSend 应用
2. 点击「发送」标签
3. 选择要发送的内容类型（文件、文件夹、剪贴板、文本、媒体、应用）
4. 选择发送模式为「P2P 传输」
5. 在设备列表中选择接收设备（通过 mDNS 自动发现）
6. 点击发送，等待对方接受

### Web 下载（链接分享）

1. 打开 PureSend 应用
2. 点击「发送」标签，选择发送模式为「Web 下载」
3. 选择要分享的文件
4. 系统生成下载链接和二维码
5. 将链接或二维码分享给接收方，对方在浏览器中打开即可下载

### 接收文件

1. 打开 PureSend 应用
2. 点击「接收」标签
3. 选择接收模式（P2P 接收或 Web 上传接收）
4. P2P 模式下会自动监听来自局域网内其他设备的传输请求
5. Web 上传模式下生成上传链接，接收方在浏览器中上传文件
6. 收到请求后审批并接收文件

## 下一步

- 了解更多 [文件传输功能](/features/file-transfer)
- 查看 [开发指南](/development/setup)
