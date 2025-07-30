# StorageHub MSP API Endpoints - Comprehensive List

## 1. Authentication Endpoints

### POST /auth/nonce
- **Purpose**: Generate SIWE challenge message
- **Auth**: None (Public)
- **Request Body**:
```json
{
  "address": "0x...",
  "chainId": 1
}
```
- **Response (200 OK)**:
```json
{
  "message": "example.com wants you to sign in...",
  "nonce": "aBcDeF12345"
}
```

### POST /auth/verify
- **Purpose**: Verify signature & create session
- **Auth**: None (Public)
- **Request Body**:
```json
{
  "message": "...",
  "signature": "0x..."
}
```
- **Response (200 OK)**:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "address": "0x..."
  }
}
```
- **Error (401)**: Invalid signature or expired nonce

### POST /auth/refresh
- **Purpose**: Refresh JWT token
- **Auth**: Bearer Token Required
- **Response (200 OK)**:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

### POST /auth/logout
- **Purpose**: Invalidate session
- **Auth**: Bearer Token Required
- **Response**: 204 No Content

### GET /auth/profile
- **Purpose**: Get user profile info
- **Auth**: Bearer Token Required
- **Response (200 OK)**:
```json
{
  "address": "0x...",
  "ens": "user.eth"
}
```

## 2. MSP Information Endpoints

### GET /info
- **Purpose**: MSP metadata and configuration
- **Auth**: None (Public)
- **Response (200 OK)**:
```json
{
  "client": "storagehub-node v1.0.0",
  "version": "StorageHub MSP v0.1.0",
  "mspId": "4c310f61f81475048e8ce5eadf4ee718c42ba285579bb37ac6da55a92c638f42",
  "multiaddresses": ["/ip4/192.168.0.10/tcp/30333/p2p/12D3KooW..."],
  "ownerAccount": "0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",
  "paymentAccount": "0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",
  "status": "active",
  "activeSince": 123,
  "uptime": "2 days, 1 hour"
}
```

### GET /stats
- **Purpose**: Real-time usage statistics
- **Auth**: None (Public)
- **Response (200 OK)**:
```json
{
  "capacity": {
    "totalBytes": 1099511627776,
    "availableBytes": 879609302220,
    "usedBytes": 219902325556
  },
  "activeUsers": 152,
  "lastCapacityChange": 123,
  "valuePropsAmount": 42,
  "BucketsAmount": 1024
}
```

### GET /value-props
- **Purpose**: Available storage plans
- **Auth**: None (Public)
- **Response (200 OK)**:
```json
[
  {
    "id": "f32282ba18056b02cf2feb4cea92aa4552131617cdb7da03acaa554e4e736c32",
    "pricePerGbBlock": 0.5,
    "dataLimitPerBucketBytes": 10737418240,
    "isAvailable": true
  }
]
```

### GET /health
- **Purpose**: System health status
- **Auth**: None (Public)
- **Response (200 OK)**:
```json
{
  "status": "healthy",
  "components": {
    "database": {
      "status": "healthy",
      "details": "PostgreSQL connection active"
    },
    "mspClient": {
      "status": "healthy",
      "details": "Connected to StorageHub MSP client"
    },
    "storageHubNetwork": {
      "status": "healthy",
      "details": "Node synced with network"
    }
  },
  "lastChecked": "2025-07-01T15:52:00.000Z"
}
```
- **Error (503)**: If any critical component is unhealthy

## 3. Bucket Management Endpoints

### GET /buckets
- **Purpose**: List user's buckets
- **Auth**: Bearer Token Required
- **Response (200 OK)**:
```json
[
  {
    "bucketId": "d8793e4187f5642e96016a96fb33849a7e03eda91358b311bbd426ed38b26692",
    "name": "Documents",
    "root": "3de0c6d1959ece558ec030f37292e383a9c95f497e8235b89701b914be9bd1fb",
    "isPublic": false,
    "sizeBytes": 12345678,
    "valuePropId": "f32282ba18056b02cf2feb4cea92aa4552131617cdb7da03acaa554e4e736c32",
    "fileCount": 12
  }
]
```

### GET /buckets/:bucketId
- **Purpose**: Get bucket metadata
- **Auth**: Bearer Token Required
- **Path Params**: `bucketId` (string)
- **Response (200 OK)**: Same structure as individual bucket in list

### GET /buckets/:bucketId/files
- **Purpose**: Get file tree structure
- **Auth**: Bearer Token Required
- **Path Params**: `bucketId` (string)
- **Response (200 OK)**:
```json
{
  "name": "/",
  "type": "folder",
  "children": [
    {
      "name": "Thesis",
      "type": "folder",
      "children": [
        {
          "name": "Initial_results.png",
          "type": "file",
          "sizeBytes": 54321,
          "fileKey": "d298c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f2"
        }
      ]
    }
  ]
}
```

## 4. File Operations Endpoints

### GET /buckets/:bucketId/:fileLocation
- **Purpose**: Download file by location/path
- **Auth**: Bearer Token Required
- **Path Params**: 
  - `bucketId` (string)
  - `fileLocation` (string, e.g., "Thesis/Initial_results.png")
- **Response (200 OK)**: Raw file data
- **Error (404)**: Bucket/file not found

### GET /buckets/:bucketId/:fileKey
- **Purpose**: Download file by file key
- **Auth**: Bearer Token Required
- **Path Params**:
  - `bucketId` (string)
  - `fileKey` (string)
- **Response (200 OK)**: Raw file data
- **Error (404)**: Bucket/file not found

### GET /buckets/:bucketId/:fileKey/info
- **Purpose**: Get file metadata
- **Auth**: Bearer Token Required
- **Path Params**:
  - `bucketId` (string)
  - `fileKey` (string)
- **Response (200 OK)**:
```json
{
  "fileKey": "d298c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f2",
  "fingerprint": "5d7a3700e1f7d973c064539f1b18c988dace6b4f1a57650165e9b58305db090f",
  "bucketId": "d8793e4187f5642e96016a96fb33849a7e03eda91358b311bbd426ed38b26692",
  "name": "Q1-2024.pdf",
  "location": "/files/documents/reports",
  "size": 54321,
  "isPublic": true,
  "uploadedAt": "2025-07-01T15:42:18.123456Z"
}
```

### PUT /buckets/:bucketId/:fileKey/upload
- **Purpose**: Upload file after on-chain storage request
- **Auth**: Bearer Token Required
- **Path Params**:
  - `bucketId` (string)
  - `fileKey` (string)
- **Request**: multipart/form-data with file
- **Response (201 Created)**: File metadata (same as info endpoint)
- **Errors**:
  - 403: Storage request not for current user
  - 404: Bucket/storage request not found

### POST /buckets/:bucketId/:fileKey/distribute
- **Purpose**: Request MSP to distribute file to volunteering BSPs
- **Auth**: Bearer Token Required
- **Path Params**:
  - `bucketId` (string)
  - `fileKey` (string)
- **Response (200 OK)**:
```json
{
  "status": "distribution_initiated",
  "fileKey": "d298c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f2",
  "message": "File distribution to volunteering BSPs has been initiated"
}
```
- **Errors**:
  - 403: Not authorized to distribute this file
  - 404: File not found
  - 409: Storage request already closed (cannot distribute)

## 5. Utility Endpoints

### GET /payment_stream
- **Purpose**: Get payment stream info between user and MSP
- **Auth**: Bearer Token Required
- **Response (200 OK)**:
```json
{
  "tokensPerBlock": 100,
  "lastChargedTick": 1234567,
  "userDeposit": 100000,
  "outOfFundsTick": null
}
```

## Summary

### Authentication Flow
1. **SIWE (Sign-In with Ethereum)**: Uses EIP-4361 standard
2. **JWT Sessions**: Bearer token authentication after login

### Access Control
- **Public Endpoints**: MSP info, health, stats, value-props
- **Auth Required**: All bucket/file operations, profile, payment stream

### Data Types & Identifiers
- **Addresses**: Ethereum addresses (0x...)
- **Bucket IDs**: 64-char hex strings
- **File Keys**: 64-char hex strings
- **File Locations**: Path strings (e.g., "Documents/report.pdf")

### Special Considerations for Mock Implementation
1. **File Operations**: Support both file key and path-based access
2. **Tree Structure**: Files endpoint returns nested JSON tree
3. **Multipart Forms**: Upload and distribute endpoints
4. **Raw File Data**: Download endpoints return binary data
5. **Error Responses**: Consistent 403/404/409 patterns