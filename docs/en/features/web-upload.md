# Web Upload

The Web Upload feature allows other devices to upload files to your PureSend app via a browser, without requiring the sender to install the app.

## How to Use

### Enable Web Upload

1. On the "Receive" page, switch the receive mode to "Web Upload"
2. The system will start an HTTP upload server
3. The generated upload link and QR code will be displayed

### Receive Files

- Share the upload link or QR code with the sender
- The sender opens the link in a browser
- The sender selects files and uploads them

### Sender (Browser) Operations

1. Open the upload link in a browser
2. Click "Select Files" or drag and drop files to the upload area
3. Confirm the file list and click "Upload"
4. Wait for the upload to complete

## Access Control

### IP Approval

- New IP addresses are required to send an access request on first visit
- Receiver can choose to "Accept" or "Reject" the IP's upload permission
- Authorized IPs can upload directly without re-approval

## Transfer Features

- **Chunked Upload**: Large files are automatically chunked with resume support
- **Encrypted Transfer**: AES-256-GCM encryption to protect data in transit
- **Progress Display**: Real-time upload progress on both browser and app sides
- **File Overwrite**: Configurable overwrite policy for files with the same name

## Notes

- Receiver and sender need to be on the same LAN or mutually accessible network
- PureSend must remain running during upload reception
- Uploaded files are saved to the configured receive directory
