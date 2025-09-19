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

  // Small helpers to simplify state updates
  const setState = (partial: Partial<CheckpointState>) => update(s => ({ ...s, ...partial }));
  const setLoading = (isLoading: boolean, lastError: string | null = null) => setState({ isLoading, lastError });

  // Simple TTL cache + in-flight dedupe to avoid spamming the API
  const cache = new Map<string, { at: number; data: CheckpointMeta[] }>();
  const inflight = new Map<string, Promise<CheckpointMeta[]>>();
  const TTL_MS = 5_000; // 5s
  const autoSelected = new Set<string>(); // namespaces auto-selected this session
  let selectCounter = 0; // concurrency token for restore-on-select

  async function list(namespace?: string) {
    const ns = namespace || get({ subscribe }).activeNamespace;
    const key = `list:${ns}`;

    // Serve from fresh cache
    const cached = cache.get(key);
    if (cached && Date.now() - cached.at < TTL_MS) {
      setState({ list: cached.data });
      return cached.data;
    }

    // Return existing in-flight promise if any
    const existing = inflight.get(key);
    if (existing) return existing;

    const adapter = getCheckpointAdapter();

    const p = (async () => {
      setLoading(true, null);
      try {
        const checkpoints = await adapter.list(ns);
        cache.set(key, { at: Date.now(), data: checkpoints });
        setState({ list: checkpoints, isLoading: false });
        // Auto-select latest id once per namespace (no restore side-effects)
        const current = get({ subscribe });
        if (!current.selectedId && checkpoints.length > 0 && !autoSelected.has(ns)) {
          autoSelected.add(ns);
          setState({ selectedId: checkpoints[0].id });
        }
        return checkpoints;
      } catch (e: any) {
        const msg = `Failed to list checkpoints: ${e?.message || e}`;
        console.error('[CheckpointStore]', msg);
        setLoading(false, msg);
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
    setLoading(true, null);
    try {
      const checkpoint = await adapter.create(ns, metadata);
      // Invalidate cache for this namespace and refresh list from source to avoid race conditions
      cache.delete(`list:${ns}`);
      const refreshed = await list(ns);
      const isAgent = (metadata?.actor || '').toLowerCase() === 'agent';
      // Select new checkpoint unless created by agent (agent flows may keep current selection)
      if (!isAgent) {
        setState({ selectedId: checkpoint.id });
      }
      setLoading(false, null);
      return checkpoint;
    } catch (e: any) {
      const msg = `Failed to create checkpoint: ${e?.message || e}`;
      console.error('[CheckpointStore]', msg);
      setLoading(false, msg);
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
    setLoading(true, null);
    try {
      const res = await adapter.restore(ns, id);
      // Invalidate cache; underlying history changed
      cache.delete(`list:${ns}`);
      setLoading(false, null);
      // Optional: emit a clean UI event
      eventStore.emit({ type: 'ui', subtype: 'project_changed', payload: { projectPath: 'checkpoint_restored', timestamp: Date.now() } });
      return res;
    } catch (e: any) {
      const msg = `Failed to restore checkpoint: ${e?.message || e}`;
      console.error('[CheckpointStore]', msg);
      setLoading(false, msg);
      throw e;
    }
  }

  async function publish() {
    const ns = get({ subscribe }).activeNamespace;
    const adapter = getCheckpointAdapter();
    setLoading(true, null);
    try {
      const res = await adapter.publish(ns);
      cache.delete(`list:${ns}`);
      setLoading(false, null);
      eventStore.emit({ type: 'ui', subtype: 'project_changed', payload: { projectPath: 'checkpoint_published', timestamp: Date.now() } });
      return res;
    } catch (e: any) {
      const msg = `Failed to publish checkpoints: ${e?.message || e}`;
      console.error('[CheckpointStore]', msg);
      setLoading(false, msg);
      throw e;
    }
  }

  function setActiveNamespace(ns: string) {
    update(s => ({ ...s, activeNamespace: ns }));
  }

  async function select(id: string | null) {
    const prev = get({ subscribe }).selectedId;
    if (id === null) {
      setState({ selectedId: null });
      return;
    }
    const token = ++selectCounter;
    setLoading(true, null);
    try {
      await restore(id);
      // Only apply if this is the latest select
      if (token === selectCounter) {
        setState({ selectedId: id, isLoading: false });
      }
    } catch (e: any) {
      const msg = `Failed to restore checkpoint: ${e?.message || e}`;
      console.error('[CheckpointStore]', msg);
      if (token === selectCounter) {
        setState({ selectedId: prev ?? null });
        setLoading(false, msg);
      }
    }
  }

  function setComparingTo(id: string | null) {
    update(s => ({ ...s, comparingToId: id }));
  }

  return { subscribe, list, create, diff, restore, publish, setActiveNamespace, select, setComparingTo,
    getCurrentState() { return get({ subscribe }); } };
}

export const checkpointStore = createCheckpointStore();
