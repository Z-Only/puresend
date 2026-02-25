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

### Sending Files

1. Open the PureSend app
2. Click the "Send" tab
3. Select the files or folders you want to send
4. Wait for the receiver to scan the QR code or enter the pairing code

### Receiving Files

1. Open the PureSend app
2. Click the "Receive" tab
3. Scan the sender's QR code, or enter the pairing code
4. Wait for the connection to be established
5. Choose the save location and start receiving

## Next Steps

- Learn more about [File Transfer](/features/file-transfer)
- Check out the [Development Guide](/development/setup)
