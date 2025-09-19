import { apiClient } from '../../client';
import type { CheckpointMeta, CheckpointDiffResult, CheckpointOperations, GitDiff } from '../../types';

export class WebCheckpointAdapter implements CheckpointOperations {
  async list(namespace: string): Promise<CheckpointMeta[]> {
    const params = new URLSearchParams({ ns: namespace });
    const res = await apiClient.get<{ checkpoints: any[] }>(`/api/checkpoints?${params}`);
    return (res.checkpoints || []).map((cp: any) => ({
      id: cp.id,
      namespace: cp.namespace,
      title: cp.title,
      bullets: cp.bullets,
      createdAt: cp.createdAt ?? cp.created_at ?? cp.createdAt,
      author: cp.author,
    } as CheckpointMeta));
  }
  async create(namespace: string, metadata?: { reason?: string; actor?: string; paths?: string[] }): Promise<CheckpointMeta> {
    const params = new URLSearchParams({ ns: namespace });
    const res = await apiClient.post<{ checkpoint: CheckpointMeta }>(`/api/checkpoints?${params}`, metadata || {});
    return res.checkpoint;
  }
  async diff(namespace: string, baseId: string, targetRef: 'HEAD' | 'WORKTREE' | string = 'HEAD'): Promise<CheckpointDiffResult> {
    const params = new URLSearchParams({ ns: namespace, base: baseId, target: String(targetRef) });
    const res = await apiClient.get<{ diff: GitDiff }>(`/api/checkpoints/diff?${params}`);
    return { diff: res.diff };
  }
  async restore(namespace: string, id: string): Promise<{ result: string; commitId?: string }> {
    const params = new URLSearchParams({ ns: namespace, id });
    return await apiClient.post<{ result: string; commitId?: string }>(`/api/checkpoints/restore?${params}`, {});
  }
  async publish(namespace: string): Promise<{ result: string; mergeCommitId?: string }> {
    const params = new URLSearchParams({ ns: namespace });
    return await apiClient.post<{ result: string; mergeCommitId?: string }>(`/api/checkpoints/publish?${params}`, {});
  }
}
