# Web Download (Link Sharing)

The Web Download feature allows you to share files via HTTP links. Recipients can download files directly in any browser without installing PureSend.

## How to Use

### Enable Web Download

1. On the "Send" page, switch the send mode to "Web Download"
2. Select the files to share
3. Click start sharing - the system will start an HTTP server
4. The generated download link and QR code will be displayed

### Share Files

- **Link Sharing**: Copy the download link and send it to the receiver
- **QR Code Sharing**: Let the receiver scan the QR code to open the download page

### Receiver Operations

1. Open the download link in a browser
2. Browse the available file list
3. Click a file name to download

## Access Control

### PIN Protection

- Set a PIN code in sharing settings
- Receivers need to enter the correct PIN to access the file list
- Failed PIN attempts are limited; excessive attempts result in temporary lockout

### Access Approval

- New devices are required to send an access request on first visit
- Sender can choose to "Accept" or "Reject"
- Auto-accept mode can be enabled to skip manual approval

## Security Features

- AES-256-GCM encrypted transfer support
- Browser uses Web Crypto API for P-256 ECDH key exchange
- File data is transmitted only within the local network
- Download links become invalid immediately after sharing is stopped

## Notes

- Sender and receiver need to be on the same LAN or mutually accessible network
- PureSend must remain running during sharing
- Multiple devices can download simultaneously
