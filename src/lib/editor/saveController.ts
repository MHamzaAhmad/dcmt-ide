import type * as Monaco from 'monaco-editor';
import { fileSystemApi } from '$lib/api/adapters';
import { workspaceStore } from '$lib/stores/workspace';

type SaveFn = (path: string) => Promise<void>;

export class SaveController {
  private path: string;
  private model: Monaco.editor.ITextModel;
  private saveFn: SaveFn | null;
  private debounceMs: number;
  private timer: any = null;
  private inFlight: Promise<void> | null = null;
  private pending: boolean = false;
  private disposed = false;
  private lastSavedVersionId: number;

  constructor(opts: { path: string; model: Monaco.editor.ITextModel; saveFn?: SaveFn; debounceMs?: number }) {
    this.path = opts.path;
    this.model = opts.model;
  this.saveFn = opts.saveFn ?? null;
    this.debounceMs = opts.debounceMs ?? 800;
    this.lastSavedVersionId = this.model.getVersionId();
  }

  schedule() {
    if (this.disposed) return;
    // Skip if model hasn't changed since last save
    if (this.model.getVersionId() === this.lastSavedVersionId) return;

    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => this.run(), this.debounceMs);
  }

  async flush() {
    if (this.disposed) return;
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    await this.run();
  }

  private async run() {
    if (this.disposed) return;
    if (this.model.getVersionId() === this.lastSavedVersionId) return;

    if (this.inFlight) {
      this.pending = true;
      return;
    }

    const runOnce = async () => {
      try {
        if (this.saveFn) {
          await this.saveFn(this.path);
        } else {
          // Default: persist via platform API, then commit to workspace store
          const content = this.model.getValue();
          await fileSystemApi.writeFileContent(this.path, content);
          workspaceStore.commitSave(this.path, content, 'user');
        }
        this.lastSavedVersionId = this.model.getVersionId();
      } finally {
        // no-op
      }
    };

    this.inFlight = runOnce();
    try {
      await this.inFlight;
    } finally {
      this.inFlight = null;
      if (this.pending) {
        this.pending = false;
        // If changed again since last save, schedule immediately
        if (this.model.getVersionId() !== this.lastSavedVersionId) {
          this.timer = setTimeout(() => this.run(), this.debounceMs);
        }
      }
    }
  }

  dispose() {
    this.disposed = true;
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
    this.inFlight = null;
  }
}
