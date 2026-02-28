# Getting Started

This guide will help you get started with PureSend quickly.

## System Requirements

### Desktop

| Platform | Minimum Version |
| ------- | ------------- |
| macOS   | macOS 10.13   |
| Windows | Windows 7     |
| Linux   | Major distributions    |

### Mobile

| Platform | Minimum Version          |
| ------- | ----------------- |
| Android | API 24 (Android 7.0) |

## Installation

### Download the App

Visit the [GitHub Releases](https://github.com/Z-Only/puresend/releases) page to download the installer for your platform:

- **macOS**: Download `.dmg` file
- **Windows**: Download `.msi` or `.exe` installer
- **Linux**: Download `.deb`, `.rpm`, or `.AppImage` file
- **Android**: Download `.apk` file

### Installation Steps

#### macOS

1. Open the downloaded `.dmg` file
2. Drag PureSend to the Applications folder
3. On first run, you may need to allow it in "System Preferences > Security & Privacy"

#### Windows

1. Run the downloaded `.msi` or `.exe` installer
2. Follow the installation wizard to complete the installation

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

## Basic Usage

### P2P Direct Transfer

1. Open the PureSend app
2. Click the "Send" tab
3. Select the content type (files, folders, clipboard, text, media, app)
4. Set the transfer mode to "P2P Transfer"
5. Select the receiving device from the device list (auto-discovered via mDNS)
6. Click send and wait for the receiver to accept

### Web Download (Link Sharing)

1. Open the PureSend app
2. Click the "Send" tab and set the transfer mode to "Web Download"
3. Select the files to share
4. The system generates a download link and QR code
5. Share the link or QR code with the receiver, who can download files in any browser

### Receiving Files

1. Open the PureSend app
2. Click the "Receive" tab
3. Select the receive mode (P2P Receive or Web Upload Receive)
4. In P2P mode, it automatically listens for transfer requests from other devices on the LAN
5. In Web Upload mode, a link is generated for the sender to upload files via browser
6. Review and accept incoming transfer requests

## Next Steps

- Learn more about [File Transfer](/features/file-transfer)
- Check out the [Development Guide](/development/setup)
