# API Adapters Structure

This directory contains platform-specific API adapters organized by platform and entity.

## Structure

```
adapters/
├── desktop/                    # Desktop (Tauri) platform
│   ├── filesystem.ts          # File system operations
│   ├── project.ts             # Project management
│   └── index.ts               # Combined desktop adapter
├── web/                       # Web (HTTP) platform  
│   ├── filesystem.ts          # HTTP-based file operations
│   ├── project.ts             # Limited project management
│   ├── websocket.ts           # Real-time file watching
│   └── index.ts               # Combined web adapter
└── index.ts                   # Unified platform adapter

```

## Design Principles

1. **Entity-based separation**: Each file handles one specific domain (filesystem, project, websocket)
2. **Platform-specific**: Desktop and web have separate implementations
3. **Composition over delegation**: Uses spread operator and binding instead of method delegation
4. **Interface compliance**: All adapters implement specific interfaces for type safety
5. **Future-ready**: Generic `PlatformAPI` can easily extend to support compilation, debugging, etc.

## Usage

```typescript
import { platformApi } from '$lib/api/adapters';

// Automatically uses correct adapter based on platform
const files = await platformApi.getDirectoryTree('');
const project = await platformApi.getCurrentProject();
```

## Adding New APIs

To add new API categories (e.g., compilation):

1. Define interface in `types.ts`:
   ```typescript
   export interface CompilationOperations {
     compile(files: string[]): Promise<CompilationResult>;
   }
   ```

2. Add to platform adapters:
   ```typescript
   // desktop/compilation.ts
   export class DesktopCompilationAdapter implements CompilationOperations
   ```

3. Include in platform index files using spread operator
4. Update `PlatformAPI` interface to extend the new operations