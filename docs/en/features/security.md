# Transfer Security

PureSend provides multiple layers of security to protect your file transfers.

## Transfer Encryption

### AES-256-GCM

- All transfer data can be encrypted using AES-256-GCM
- Each encryption session uses an independent key
- Encryption can be enabled/disabled in settings

### Key Exchange

PureSend uses different key exchange algorithms for different transfer modes:

- **P2P Transfer**: Uses X25519 key exchange
- **Web Transfer (Browser)**: Uses P-256 ECDH key exchange, compatible with browser Web Crypto API

### Encryption Flow

1. Client and server each generate temporary key pairs
2. Negotiate a shared secret via key exchange protocol
3. Derive AES-256 encryption key from the shared secret using HKDF
4. Transfer data is encrypted/decrypted using AES-256-GCM with the derived key

## Dynamic Compression

### zstd Compression

- High-performance compression based on the zstd (Zstandard) algorithm
- Can be enabled/disabled in settings
- Supports adjustable compression levels

### Smart Compression Mode

- **Auto mode**: Automatically determines whether to compress based on file type
- Skips already-compressed files (e.g., .zip, .jpg, .mp4)
- Applies compression to text and document files, significantly reducing transfer data

## Access Control

### PIN Protection

- Web download links can be protected with a 4-6 digit PIN code
- Failed PIN attempt limits with temporary 5-minute lockout
- Effectively prevents unauthorized access

### Access Approval

- P2P transfer: Receiver can approve or reject transfer requests
- Web download: Sender can approve or reject download requests
- Web upload: Receiver can approve upload permissions per IP

## Privacy Protection

- All data is transferred only within the LAN, not through external servers
- Privacy mode completely disables transfer history
- Web download/upload links become invalid immediately after sharing is stopped

## Settings

In the "Advanced" section of app settings, you can configure:

- Enable/disable transfer encryption
- Enable/disable dynamic compression
- Adjust compression level and mode
