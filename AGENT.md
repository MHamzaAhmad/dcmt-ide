# Agent System Documentation

## Overview

The Agent System is a LaTeX-focused AI assistant that integrates with LiteLLM to provide intelligent document creation and editing capabilities. The system supports tool calling, concurrent sessions, real-time progress updates via WebSocket, and parallel tool execution for optimal performance.

## Architecture

### Clean Architecture Pattern

The system follows a clean architecture pattern with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────┐
│                    Transport Layer                           │
│  ┌─────────────┐ ┌──────────────┐ ┌─────────────────────┐   │
│  │   Routes    │ │   Handlers   │ │    Middleware       │   │
│  │             │ │              │ │                     │   │
│  └─────────────┘ └──────────────┘ └─────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                     Service Layer                           │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                AgentService                             │ │
│  │                                                         │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                   Repository Layer                          │
│  ┌─────────────┐ ┌──────────────┐ ┌─────────────────────┐   │
│  │  AgentRepo  │ │SessionManager│ │ EventBroadcaster    │   │
│  │             │ │              │ │                     │   │
│  └─────────────┘ └──────────────┘ └─────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                      Model Layer                            │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              Data Models & Types                        │ │
│  │                                                         │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Core Components

#### 1. Agent Repository (`src/repo/agent/mod.rs`)
- **Purpose**: Core business logic for agent operations
- **Responsibilities**:
  - LiteLLM API integration
  - Tool execution orchestration
  - Parallel tool execution
  - Session management integration
  - Real-time event broadcasting

#### 2. Session Manager (`src/repo/agent/session.rs`)
- **Purpose**: Thread-safe session management
- **Features**:
  - Concurrent user support with `DashMap`
  - Automatic session cleanup
  - Message history tracking
  - Session information retrieval

#### 3. Event Broadcaster (`src/repo/agent/events.rs`)
- **Purpose**: Real-time progress updates via WebSocket
- **Features**:
  - Session-specific event subscriptions
  - Automatic cleanup of stale subscriptions
  - Support for parallel tool execution events

#### 4. Tool System (`src/repo/agent/tools/`)
- **Purpose**: Modular and extensible tool architecture
- **Design**:
  - Each tool in separate file for maintainability
  - Common `AgentTool` trait for consistency
  - Automatic tool registration via `ToolRegistry`

#### 5. Agent Service (`src/svc/agent_service.rs`)
- **Purpose**: Service layer with job queue management
- **Features**:
  - Asynchronous job processing
  - Background task execution
  - Queue monitoring and management

## API Endpoints

### POST `/api/agent/chat`

Main endpoint for agent interactions.

**Request:**
```json
{
  "session_id": "unique-session-id",
  "message": "Create a LaTeX document with title 'My Paper'",
  "model": "gpt-4-turbo"
}
```

**Response:**
```json
{
  "session_id": "unique-session-id",
  "message": "Request queued for processing. You'll receive updates via WebSocket.",
  "job_id": "job-uuid"
}
```

**Features:**
- Non-streaming API (progress via WebSocket)
- Immediate response with job ID
- Background processing with tool calling
- Session history maintained automatically

### GET `/api/agent/tools`

Returns information about available tools.

**Response:**
```json
{
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "read_file",
        "description": "Read the contents of a file from the workspace",
        "parameters": {
          "type": "object",
          "properties": {
            "path": {
              "type": "string",
              "description": "Relative path to the file within workspace"
            }
          },
          "required": ["path"]
        }
      }
    }
  ],
  "count": 6
}
```

### GET `/api/agent/session/{session_id}`

Returns information about a specific session.

**Response:**
```json
{
  "session": {
    "id": "session-id",
    "user_id": null,
    "message_count": 5,
    "created_at": "...",
    "last_activity": "..."
  },
  "found": true
}
```

## Available Tools

All tools operate within the workspace directory and include security measures to prevent path traversal attacks.

### 1. Read File Tool (`read_file`)
- **Purpose**: Read file contents from workspace
- **Parameters**: `path` (relative path)
- **Security**: Path validation, existence checks
- **Output**: File contents with metadata

### 2. Write File Tool (`write_file`)
- **Purpose**: Create or overwrite files
- **Parameters**: `path`, `content`
- **Features**: Automatic directory creation
- **Output**: Success confirmation with file stats

### 3. Update File Tool (`update_file`)
- **Purpose**: Targeted file modifications using find-and-replace
- **Parameters**: `path`, `old_content`, `new_content`
- **Safety**: Exact matching required, unique content validation
- **Output**: Modification summary

### 4. List Files Tool (`list_files`)
- **Purpose**: Directory exploration
- **Parameters**: `path` (default: "."), `show_hidden`
- **Features**: File size reporting, directory structure
- **Output**: Organized file and directory listing

### 5. Create Directory Tool (`create_directory`)
- **Purpose**: Directory creation
- **Parameters**: `path`
- **Features**: Recursive directory creation
- **Output**: Creation confirmation

### 6. Delete File Tool (`delete_file`)
- **Purpose**: File deletion with safety measures
- **Parameters**: `path`, `confirm` (must be true)
- **Safety**: Explicit confirmation required
- **Output**: Deletion confirmation with file metadata

## WebSocket Events

The system provides real-time progress updates via WebSocket. Clients should connect to the WebSocket endpoint and subscribe to session-specific events.

### Event Types

```typescript
type AgentEvent = 
  | { type: "JobQueued", job_id: string, session_id: string }
  | { type: "LLMCallStart", model: string }
  | { type: "LLMStreaming", content: string }
  | { type: "ToolCallRequested", tool: string, args: object }
  | { type: "ToolExecuting", tool: string }
  | { type: "ToolCompleted", tool: string, result: string }
  | { type: "ParallelToolsStart", count: number }
  | { type: "ParallelToolsComplete", count: number }
  | { type: "LLMCallComplete" }
  | { type: "JobComplete", response: string }
  | { type: "Error", message: string };
```

### WebSocket Integration

The existing WebSocket handler has been enhanced to support agent events. Clients need to subscribe to their session ID to receive events.

## System Prompt

The system uses a specialized LaTeX-focused prompt loaded from `config/systemprompt.md`. The prompt emphasizes:

- LaTeX best practices and conventions
- Academic writing standards
- Error prevention strategies
- Tool usage guidelines
- Educational explanations

## Configuration

### Environment Variables

Add to your `.env` file:

```bash
# LiteLLM API base URL
LITELLM_BASE_URL=http://0.0.0.0:4000
```

### LiteLLM Integration

The system integrates with LiteLLM proxy server:

1. **All Tools Included**: Every request includes all available tool definitions
2. **Structured JSON**: Uses JSON response format for reliable parsing
3. **Error Handling**: Comprehensive error handling for API failures
4. **Authentication**: Passes through authentication headers

## Key Features

### 1. Parallel Tool Execution

When the LLM requests multiple tool calls simultaneously, the system executes them in parallel using `futures::future::join_all()`, significantly improving performance.

```rust
// Multiple tools - execute in parallel
let tool_futures: Vec<_> = tool_calls
    .iter()
    .map(|tool_call| self.execute_tool_async(tool_call.clone(), session_id))
    .collect();

let results = join_all(tool_futures).await;
```

### 2. Session Management

- **Thread-Safe**: Uses `DashMap` for concurrent access
- **Automatic Cleanup**: Background task removes expired sessions
- **Message History**: Maintains full conversation context
- **Multi-User Support**: Ready for user-specific sessions

### 3. Real-Time Progress

- **WebSocket Integration**: Seamless integration with existing WebSocket system
- **Event Broadcasting**: Session-specific event delivery
- **Progress Tracking**: Detailed progress information for complex operations
- **Error Reporting**: Real-time error notifications

### 4. Extensible Tool System

Adding new tools is straightforward:

1. Create new tool file in `src/repo/agent/tools/`
2. Implement `AgentTool` trait
3. Add to `ToolRegistry::new()`
4. Tool automatically available to LLM

### 5. Clean Architecture Benefits

- **Testability**: Each layer can be tested independently
- **Maintainability**: Clear separation of concerns
- **Extensibility**: Easy to add new features
- **Scalability**: Repository pattern supports different data sources

## Usage Examples

### Basic Chat Request

```bash
curl -X POST http://localhost:3001/api/agent/chat \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "user-session-123",
    "message": "Create a basic LaTeX article with title My Research Paper",
    "model": "gpt-4-turbo"
  }'
```

### WebSocket Subscription (JavaScript)

```javascript
const ws = new WebSocket('ws://localhost:3001/ws');

ws.onopen = () => {
  // Subscribe to session events
  ws.send(JSON.stringify({
    type: "subscribe_agent",
    session_id: "user-session-123"
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  if (data.type === "agent_event") {
    handleAgentEvent(data.event);
  }
};

function handleAgentEvent(event) {
  switch (event.type) {
    case "ToolExecuting":
      console.log(`Executing ${event.tool}...`);
      break;
    case "ToolCompleted":
      console.log(`${event.tool} completed: ${event.result}`);
      break;
    case "JobComplete":
      console.log(`Job finished: ${event.response}`);
      break;
  }
}
```

## Performance Considerations

### 1. Parallel Execution
- Multiple tool calls execute concurrently
- Significant performance improvement for complex operations
- Real-time progress updates for all parallel operations

### 2. Session Management
- Thread-safe concurrent access with `DashMap`
- Automatic cleanup prevents memory leaks
- Efficient message history management

### 3. WebSocket Optimization
- Session-specific event delivery
- Automatic cleanup of stale subscriptions
- Minimal overhead for real-time updates

### 4. Background Processing
- Non-blocking API responses
- Asynchronous job execution
- Queue management for high load scenarios

## Error Handling

The system implements comprehensive error handling:

- **API Errors**: LiteLLM API failures with detailed error messages
- **Tool Errors**: Individual tool execution failures with recovery
- **Session Errors**: Session management error handling
- **WebSocket Errors**: Connection and subscription error management
- **Validation Errors**: Request validation with clear error messages

## Security Considerations

### 1. Path Security
- All file operations validate paths
- Prevents directory traversal attacks
- Workspace-relative paths only

### 2. Authentication
- Middleware framework ready for JWT implementation
- Currently configured to allow all requests (TODO)
- Easy to implement proper authentication

### 3. Tool Safety
- Explicit confirmation required for destructive operations
- Input validation for all tool parameters
- Workspace isolation

## Testing

The system includes comprehensive tests:

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test agent
cargo test tools
cargo test session
```

## Future Enhancements

1. **Authentication**: Implement JWT-based authentication
2. **User Management**: Add user-specific sessions and permissions
3. **Tool Marketplace**: Plugin system for custom tools
4. **Analytics**: Usage analytics and performance monitoring
5. **Caching**: Response caching for frequently used operations
6. **Rate Limiting**: API rate limiting and quota management

## Troubleshooting

### Common Issues

1. **LiteLLM Connection**: Ensure LiteLLM server is running on configured URL
2. **WebSocket Issues**: Check WebSocket connection and session subscription
3. **Tool Execution**: Verify workspace permissions and file paths
4. **Session Management**: Monitor session cleanup and memory usage

### Debugging

Enable debug logging:

```bash
RUST_LOG=dcmt_backend=debug,tower_http=debug cargo run
```

### Health Checks

- **Agent Service**: `/api/agent/tools` - Should return available tools
- **Session Management**: `/api/agent/session/{id}` - Should return session info
- **WebSocket**: Connect and subscribe to verify real-time updates

## Conclusion

The Agent System provides a robust, scalable, and extensible platform for AI-assisted LaTeX document creation. With its clean architecture, parallel execution, real-time updates, and comprehensive tool system, it offers a powerful foundation for intelligent document editing workflows.