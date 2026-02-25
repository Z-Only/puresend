# File Transfer

PureSend provides fast and secure LAN file transfer functionality.

## How It Works

PureSend uses HTTP/TCP direct connection technology for peer-to-peer transfer:

1. **Device Discovery**: Discover other devices on the same network through LAN broadcasting
2. **Connection Establishment**: Establish direct TCP connection via device IP and port
3. **Data Transfer**: File data is transferred directly between devices without passing through servers

## Features

### Sending Files

- **Supported Types**: Any file type
- **Batch Transfer**: Support selecting multiple files or entire folders at once
- **Resume Transfer**: Support resuming transfer after interruption
- **Transfer Speed**: Can reach 100Mbps+ within LAN

### Receiving Files

- **Auto Discovery**: Automatically discover nearby sending devices
- **QR Code Scanning**: Quick connection by scanning QR code
- **Pairing Code Input**: Manual connection by entering pairing code
- **Save Location**: Customize the save location for received files

## Security

### LAN Direct Connection

- File data is transferred only within the LAN
- Does not pass through any external servers
- Data transfer is secure and controllable

### Privacy Protection

- Files do not pass through any relay servers
- No records are kept after transfer completion
- Only visible to devices on the same LAN

## Tips

### Improving Transfer Speed

1. Ensure devices are on the same LAN
2. Turn off VPN and proxy
3. Use 5GHz WiFi network

### Large File Transfer

- Supports transferring very large files (GB level)
- Automatic chunked transfer
- Real-time transfer progress display

### Cross-Platform Transfer

Supports file transfer between any platforms:

- macOS ↔ Windows
- Linux ↔ Android
- Desktop ↔ Mobile
