# Cloud Storage Transfer

PureSend supports file transfer through cloud storage services, enabling cross-network and cross-device file sharing.

## Overview

The cloud storage transfer feature allows you to upload files to cloud storage and then download them on other devices. It's suitable for:

- Devices not on the same LAN
- Large file transfer across networks
- Long-term file sharing
- Asynchronous transfer when the recipient is offline

## Supported Cloud Storage Types

### WebDAV

Supports standard WebDAV protocol, compatible with various cloud storage services:

- **Jianguoyun (Nutstore)** - Popular cloud storage service in China
- **NextCloud** - Open-source private cloud solution
- **Alist** - File listing program supporting multiple cloud drives
- Other services supporting WebDAV protocol

### Aliyun OSS

Alibaba Cloud Object Storage Service, suitable for:

- Large file storage and transfer
- High-frequency file distribution
- Enterprise-level storage needs

### Aliyun Drive

Aliyun Drive Personal Edition, suitable for:

- Personal daily use
- Large capacity storage needs
- Convenient file sharing

## Features

### Multi-Account Management

- Support for multiple cloud storage accounts
- Real-time account status display (Connected, Disconnected, Invalid)
- Quick switching between accounts

### File Operations

- **Browse Directory** - View cloud storage file and directory structure
- **Upload File** - Upload local files to cloud storage
- **Download File** - Download files from cloud storage to local
- **Create Directory** - Create new directories on cloud storage

### Security Protection

- **Credential Encryption** - AES-256-GCM encryption for storing passwords, keys, and other sensitive information
- **Device Binding** - Encryption key derived from device identifier, ensuring credentials are only usable on this device
- **Secure Transfer** - All cloud storage communication uses HTTPS encryption

## Usage

### Adding Cloud Storage Account

1. Go to "Settings" page
2. Select "Cloud Accounts" option
3. Click "Add Account"
4. Select cloud storage type and fill in credentials:
   - **WebDAV**: Server URL, Username, Password
   - **Aliyun OSS**: Bucket, Region, AccessKey ID, AccessKey Secret
   - **Aliyun Drive**: Refresh Token
5. Click "Test Connection" to verify credentials
6. Save the account

### Uploading Files to Cloud Storage

1. Go to "Cloud" page
2. Select a configured cloud storage account
3. Select local files in the upload panel
4. Select or create a target directory on cloud storage
5. Start upload

### Downloading Files from Cloud Storage

1. Go to "Cloud" page
2. Select a configured cloud storage account
3. Browse cloud storage directory in the download panel
4. Select files to download
5. Choose local save location
6. Start download

## Credential Guide

### WebDAV (Jianguoyun)

1. Login to Jianguoyun web version
2. Go to "Account Info" > "Security Options"
3. Add application password
4. Use the generated password as WebDAV password

Server URL format: `https://dav.jianguoyun.com/dav/`

### Aliyun OSS

1. Login to Alibaba Cloud Console
2. Go to Object Storage OSS
3. Create or select a Bucket
4. Create AccessKey in "Access Control"

### Aliyun Drive

1. Login to Aliyun Drive web version
2. Open browser developer tools
3. Find `refresh_token` parameter in network requests
4. Or use third-party tools to obtain

## Notes

- Cloud storage transfer speed depends on network bandwidth and cloud service limitations
- Some cloud services may have traffic or storage limits
- Recommend transferring large files on Wi-Fi
- Regularly check cloud storage space
