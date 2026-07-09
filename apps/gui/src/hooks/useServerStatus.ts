import { useMemo } from "react";
import type { RuntimeSnapshot } from "../lib/types";

export function useServerStatus(runtime: RuntimeSnapshot) {
  return useMemo(
    () => ({
      isOnline: runtime.server.status === "online",
      label: runtime.server.status === "online" ? "Running" : "Offline",
      url: runtime.server.url,
      uptime: runtime.server.uptime
    }),
    [runtime.server.status, runtime.server.uptime, runtime.server.url]
  );
}
