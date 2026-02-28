# File Transfer

PureSend provides fast and secure file transfer capabilities with multiple transfer modes.

## How It Works

PureSend supports three transfer modes:

### P2P Direct Transfer

1. **Device Discovery**: Auto-discover other PureSend devices on the LAN via mDNS protocol
2. **Connection Establishment**: Establish direct HTTP connection via device IP and port
3. **Security Handshake**: Use P-256 ECDH key exchange to establish encrypted channel (optional)
4. **Data Transfer**: Files are automatically chunked with optional encryption and compression

### Web Download (Link Sharing)

1. The sender starts an HTTP server and shares files
2. A download link and QR code are generated
3. The receiver opens the link in any browser to download
4. Supports PIN protection and access approval

### Web Upload

1. The receiver starts an HTTP upload server
2. An upload link and QR code are generated
3. The sender opens the link in a browser to upload files
4. Supports per-IP approval and chunked upload

## Features

### Multiple Content Types

- **Files**: Select any type of file for transfer
- **Folders**: Select entire folders for batch transfer
- **Clipboard**: Transfer clipboard contents directly
- **Text**: Quick text message sending
- **Media**: Select photos/videos from media library
- **Apps**: Share installed apps (Android)

### Transfer Capabilities

- **Resume Transfer**: Resume interrupted transfers from the breakpoint
- **Chunked Transfer**: Large files are automatically chunked (1MB/chunk) with parallel processing
- **Dynamic Compression**: Smart compression based on zstd algorithm, automatically determines whether to compress
- **Transfer Speed**: Can reach 100Mbps+ within LAN

### Device Discovery

- **mDNS Auto-Discovery**: Automatically discover PureSend devices on the same LAN
- **Manual Add**: Support adding devices manually via IP address

## Security

### Transfer Encryption

- **AES-256-GCM** end-to-end encryption protects transfer data
- **P-256 ECDH** key exchange (HTTP transfer mode, compatible with browser Web Crypto API)
- **X25519** key exchange (P2P transfer mode)
- Encryption can be enabled/disabled in settings

### Access Control

- **PIN Protection**: Web download links can be protected with PIN code
- **Access Approval**: Support manual approval or auto-accept for transfer requests
- **IP Approval**: Web upload mode supports per-IP address approval

### Privacy Protection

- File data is transferred only within LAN or Wi-Fi direct
- No external servers or cloud relay involved
- Privacy mode available to disable transfer history

## Transfer History

- Automatically records detailed information for each transfer (direction, status, file list, duration, etc.)
- Filter by direction (send/receive) and status (completed/failed, etc.)
- Sort by time, size, etc.
- Privacy mode can disable history recording

## Tips

### Improving Transfer Speed

1. Ensure devices are on the same LAN
2. Turn off VPN and proxy
3. Use 5GHz WiFi network
4. Enable dynamic compression (especially effective for text-based files)

### Cross-Platform Transfer

Supports file transfer between any platforms:

- macOS ↔ Windows
- Linux ↔ Android
- Desktop ↔ Mobile
- Any device ↔ Browser (via Web Download/Upload)
