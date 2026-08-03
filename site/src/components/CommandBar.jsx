import { useEffect, useState } from "react";
import { copyText } from "../lib/copy";

export function CommandBar({ children, label = "Command", compact = false }) {
  const value = String(children);
  const [state, setState] = useState("idle");

  useEffect(() => {
    if (state !== "copied") return undefined;
    const timeout = setTimeout(() => setState("idle"), 1800);
    return () => clearTimeout(timeout);
  }, [state]);

  const copy = async () => {
    try {
      await copyText(value);
      setState("copied");
    } catch {
      setState("failed");
    }
  };

  return (
    <div className={compact ? "command-bar is-compact" : "command-bar"}>
      <span className="sr-only">{label}</span>
      <code>{value}</code>
      <button type="button" onClick={copy} aria-live="polite">
        {state === "copied" ? "Copied" : state === "failed" ? "Copy failed" : "Copy"}
      </button>
    </div>
  );
}
