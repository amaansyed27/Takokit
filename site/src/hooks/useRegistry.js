import { useCallback, useEffect, useState } from "react";
import { getRegistry } from "../models/registry";

export function useRegistry() {
  const [state, setState] = useState({
    status: "loading",
    registry: null,
    error: null,
  });

  const load = useCallback(async ({ force = false } = {}) => {
    setState((current) => ({ ...current, status: "loading", error: null }));
    try {
      const registry = await getRegistry({ force });
      setState({ status: "ready", registry, error: null });
    } catch (error) {
      setState({ status: "error", registry: null, error });
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  return { ...state, retry: () => load({ force: true }) };
}
