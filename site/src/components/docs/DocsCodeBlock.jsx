import { useEffect, useState } from "react";
import { copyText } from "../../lib/copy";

export function DocsCodeBlock({ children, label = "Code example" }) {
  const value = String(children);
  const [state, setState] = useState("idle");

  useEffect(() => {
    if (state !== "copied") return undefined;
    const timeout = window.setTimeout(() => setState("idle"), 1800);
    return () => window.clearTimeout(timeout);
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
    <div className="docs-code-block">
      <div className="docs-code-block__bar">
        <span>{label}</span>
        <button type="button" onClick={copy} aria-live="polite">
          {state === "copied" ? "Copied" : state === "failed" ? "Copy failed" : "Copy"}
        </button>
      </div>
      <pre tabIndex={0}><code>{value}</code></pre>
    </div>
  );
}
