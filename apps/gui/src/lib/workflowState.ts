import { useSyncExternalStore } from "react";

export type WorkflowSnapshot<T> = {
  running: boolean;
  result: T | null;
  error: string | null;
  updatedAt: number | null;
};

type Listener = () => void;

export type WorkflowStore<T> = {
  getSnapshot: () => WorkflowSnapshot<T>;
  subscribe: (listener: Listener) => () => void;
  start: () => boolean;
  succeed: (result: T) => void;
  fail: (message: string) => void;
  clear: () => void;
};

export function createWorkflowStore<T>(): WorkflowStore<T> {
  let state: WorkflowSnapshot<T> = {
    running: false,
    result: null,
    error: null,
    updatedAt: null
  };
  const listeners = new Set<Listener>();

  function publish(next: WorkflowSnapshot<T>) {
    state = next;
    listeners.forEach((listener) => listener());
  }

  return {
    getSnapshot: () => state,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    start() {
      if (state.running) return false;
      publish({ ...state, running: true, error: null, updatedAt: Date.now() });
      return true;
    },
    succeed(result) {
      publish({ running: false, result, error: null, updatedAt: Date.now() });
    },
    fail(message) {
      publish({ ...state, running: false, error: message, updatedAt: Date.now() });
    },
    clear() {
      if (state.running) return;
      publish({ running: false, result: null, error: null, updatedAt: null });
    }
  };
}

export function useWorkflowStore<T>(store: WorkflowStore<T>): WorkflowSnapshot<T> {
  return useSyncExternalStore(store.subscribe, store.getSnapshot, store.getSnapshot);
}
