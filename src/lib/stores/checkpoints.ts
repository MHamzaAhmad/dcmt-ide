import { writable, get } from 'svelte/store';
import { eventStore } from './events';
import { getCheckpointAdapter } from '$lib/api/adapters';
import type { CheckpointMeta } from '$lib/api/types';

export interface CheckpointState {
  isReady: boolean;
  namespaces: string[];
  activeNamespace: string;
  list: CheckpointMeta[];
  selectedId: string | null;
  comparingToId: string | null;
  isLoading: boolean;
  lastError: string | null;
}

function createCheckpointStore() {
  const initial: CheckpointState = {
    isReady: true,
    namespaces: ['latex-agent'],
    activeNamespace: 'latex-agent',
    list: [],
    selectedId: null,
    comparingToId: null,
    isLoading: false,
    lastError: null,
  };

  const { subscribe, update } = writable<CheckpointState>(initial);

  // Simple TTL cache + in-flight dedupe to avoid spamming the API
  const cache = new Map<string, { at: number; data: CheckpointMeta[] }>();
  const inflight = new Map<string, Promise<CheckpointMeta[]>>();
  const TTL_MS = 5_000; // 5s

  async function list(namespace?: string) {
    const ns = namespace || get({ subscribe }).activeNamespace;
    const key = `list:${ns}`;

    // Serve from fresh cache
    const cached = cache.get(key);
    if (cached && Date.now() - cached.at < TTL_MS) {
      update(s => ({ ...s, list: cached.data }));
      return cached.data;
    }

    // Return existing in-flight promise if any
    const existing = inflight.get(key);
    if (existing) return existing;

    const adapter = getCheckpointAdapter();

    const p = (async () => {
      update(s => ({ ...s, isLoading: true, lastError: null }));
      try {
        const checkpoints = await adapter.list(ns);
        cache.set(key, { at: Date.now(), data: checkpoints });
        update(s => ({ ...s, list: checkpoints, isLoading: false }));
        return checkpoints;
      } catch (e: any) {
        const msg = `Failed to list checkpoints: ${e?.message || e}`;
        console.error('[CheckpointStore]', msg);
        update(s => ({ ...s, isLoading: false, lastError: msg }));
        throw e;
      } finally {
        inflight.delete(key);
      }
    })();

    inflight.set(key, p);
    return p;
  }

  async function create(metadata?: { reason?: string; actor?: string; paths?: string[] }) {
    const ns = get({ subscribe }).activeNamespace;
    const adapter = getCheckpointAdapter();
    update(s => ({ ...s, isLoading: true, lastError: null }));
    try {
      const checkpoint = await adapter.create(ns, metadata);
      // Invalidate cache for this namespace
      cache.delete(`list:${ns}`);
      update(s => ({ ...s, list: [checkpoint, ...s.list], isLoading: false, selectedId: checkpoint.id }));
      return checkpoint;
    } catch (e: any) {
      const msg = `Failed to create checkpoint: ${e?.message || e}`;
      console.error('[CheckpointStore]', msg);
      update(s => ({ ...s, isLoading: false, lastError: msg }));
      throw e;
    }
  }

  async function diff(baseId: string, targetRef: 'HEAD' | 'WORKTREE' | string = 'HEAD') {
    const ns = get({ subscribe }).activeNamespace;
    const adapter = getCheckpointAdapter();
    try {
      return await adapter.diff(ns, baseId, targetRef);
    } catch (e: any) {
      const msg = `Failed to diff checkpoint: ${e?.message || e}`;
      console.error('[CheckpointStore]', msg);
      update(s => ({ ...s, lastError: msg }));
      throw e;
    }
  }

  async function restore(id: string) {
    const ns = get({ subscribe }).activeNamespace;
    const adapter = getCheckpointAdapter();
    update(s => ({ ...s, isLoading: true, lastError: null }));
    try {
      const res = await adapter.restore(ns, id);
      // Invalidate cache; underlying history changed
      cache.delete(`list:${ns}`);
      update(s => ({ ...s, isLoading: false }));
      // Optional: emit a clean UI event
      eventStore.emit({ type: 'ui', subtype: 'project_changed', payload: { projectPath: 'checkpoint_restored', timestamp: Date.now() } });
      return res;
    } catch (e: any) {
      const msg = `Failed to restore checkpoint: ${e?.message || e}`;
      console.error('[CheckpointStore]', msg);
      update(s => ({ ...s, isLoading: false, lastError: msg }));
      throw e;
    }
  }

  async function publish() {
    const ns = get({ subscribe }).activeNamespace;
    const adapter = getCheckpointAdapter();
    update(s => ({ ...s, isLoading: true, lastError: null }));
    try {
      const res = await adapter.publish(ns);
      cache.delete(`list:${ns}`);
      update(s => ({ ...s, isLoading: false }));
      eventStore.emit({ type: 'ui', subtype: 'project_changed', payload: { projectPath: 'checkpoint_published', timestamp: Date.now() } });
      return res;
    } catch (e: any) {
      const msg = `Failed to publish checkpoints: ${e?.message || e}`;
      console.error('[CheckpointStore]', msg);
      update(s => ({ ...s, isLoading: false, lastError: msg }));
      throw e;
    }
  }

  function setActiveNamespace(ns: string) {
    update(s => ({ ...s, activeNamespace: ns }));
  }

  function select(id: string | null) {
    update(s => ({ ...s, selectedId: id }));
  }

  function setComparingTo(id: string | null) {
    update(s => ({ ...s, comparingToId: id }));
  }

  return { subscribe, list, create, diff, restore, publish, setActiveNamespace, select, setComparingTo,
    getCurrentState() { return get({ subscribe }); } };
}

export const checkpointStore = createCheckpointStore();
