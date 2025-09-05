# DCMT Backend

A Rust backend service with Axum following clean onion architecture for file system operations with real-time WebSocket updates.

## Architecture

The backend follows clean onion architecture with clear separation of concerns:

```
src/
├── cmd/           # Application entry point
├── transport/     # HTTP layer (handlers, middleware, routing)
├── svc/           # Business logic service layer
├── repo/          # Data access repository layer
└── model/         # Domain models and types
```

## Features

- **File System Service**: Sandboxed access to `/workspace` directory
- **Real-time File Watching**: Using `notify` crate for file system monitoring
- **WebSocket Broadcasting**: Real-time file change events to connected clients
- **RESTful API**: Complete CRUD operations for files and directories
- **Security**: Path traversal protection and workspace sandboxing

## API Endpoints

### File Operations

#### Get Directory Tree
- `GET /api/files/tree` - Get root directory structure
- `GET /api/files/tree/*path` - Get directory structure for specific path

#### File Content
- `GET /api/files/content/*path` - Read file content

#### File Management
- `POST /api/files` - Create file or directory
- `PUT /api/files/*path` - Update file content
- `DELETE /api/files/*path` - Delete file or directory
- `POST /api/files/rename/*old_path/*new_path` - Rename/move file

#### Server Status
- `GET /api/files/status` - Get server health and connection count

### WebSocket
- `WS /ws` - Real-time file system events

## WebSocket Events

The WebSocket connection provides real-time file system events:

```json
{
  "type": "file_event",
  "event": {
    "id": "uuid",
    "event_type": "Created|Modified|Deleted|Renamed",
    "path": "relative/path/to/file",
    "timestamp": 1234567890,
    "metadata": {
      "old_path": "optional_old_path",
      "new_path": "optional_new_path",
      "size": 1024,
      "is_dir": false
    }
  }
}
```

## Request/Response Formats

### Create File Request
```json
{
  "path": "relative/path/to/file",
  "is_dir": false,
  "content": "optional file content"
}
```

### Update File Request
```json
{
  "content": "updated file content"
}
```

### File Info Response
```json
{
  "path": "relative/path",
  "name": "filename.txt",
  "is_dir": false,
  "size": 1024,
  "modified": 1234567890,
  "children": null
}
```

## Security Features

- **Path Traversal Protection**: All paths are validated and sanitized
- **Workspace Sandboxing**: File access is restricted to the workspace directory
- **CORS**: Configured for cross-origin requests
- **Request Tracing**: Full HTTP request/response logging

## Running the Server

```bash
cd backend
cargo run
```

The server will start on `http://127.0.0.1:3001` with:
- REST API endpoints at `/api/*`
- WebSocket endpoint at `/ws`
- Workspace directory at `./workspace`

## Dependencies

- `axum` - Web framework
- `tokio` - Async runtime
- `serde` - Serialization
- `notify` - File system watcher
- `tower-http` - HTTP utilities
- `tracing` - Structured logging
- `anyhow` - Error handling