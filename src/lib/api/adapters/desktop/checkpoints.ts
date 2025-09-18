import type { CheckpointMeta, CheckpointDiffResult, CheckpointOperations, GitDiff } from '../../types';
import { invoke } from '@tauri-apps/api/core';

export class DesktopCheckpointAdapter implements CheckpointOperations {
  async list(namespace: string): Promise<CheckpointMeta[]> {
    const checkpoints = await invoke<CheckpointMeta[]>('dcmt_list_checkpoints', { namespace, max: 50 });
    return checkpoints.map(cp => ({
      ...cp,
      // normalize snake_case to camelCase if needed
      createdAt: (cp as any).createdAt ?? (cp as any).created_at ?? (cp as any).createdAt,
    } as any));
  }
  async create(namespace: string, metadata?: { reason?: string; actor?: string; paths?: string[] }): Promise<CheckpointMeta> {
    const checkpoint = await invoke<CheckpointMeta>('dcmt_create_checkpoint', { namespace, title: metadata?.reason });
    return ({
      ...checkpoint,
      createdAt: (checkpoint as any).createdAt ?? (checkpoint as any).created_at ?? (checkpoint as any).createdAt,
    } as any);
  }
  async diff(namespace: string, baseId: string, targetRef: 'HEAD' | 'WORKTREE' | string = 'HEAD'): Promise<CheckpointDiffResult> {
    const diff = await invoke<GitDiff>('dcmt_diff_checkpoint', { namespace, baseId, targetRef });
    return { diff };
  }
  async restore(namespace: string, id: string): Promise<{ result: string; commitId?: string }> {
    const commit = await invoke<{ sha: string }>('dcmt_restore_checkpoint', { namespace, id });
    return { result: 'ok', commitId: (commit as any).sha };
  }
  async publish(namespace: string): Promise<{ result: string; mergeCommitId?: string }> {
    const oid = await invoke<string>('dcmt_publish_checkpoint', { namespace });
    return { result: 'ok', mergeCommitId: oid };
  }
}
