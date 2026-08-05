import { useEffect, useState } from "react";
import { copyText } from "../../lib/copy";

const MODEL_REFS = ["kokoro", "whisper-tiny", "chatterbox", "rvc"];
const ROTATION_INTERVAL_MS = 2600;

export function RollingPullCommand() {
  const [index, setIndex] = useState(0);
  const [copyState, setCopyState] = useState("idle");
  const model = MODEL_REFS[index];
  const command = `tako pull ${model}`;

  useEffect(() => {
    if (
      typeof window === "undefined" ||
      window.matchMedia("(prefers-reduced-motion: reduce)").matches
    ) {
      return undefined;
    }

    const interval = window.setInterval(() => {
      setIndex((current) => (current + 1) % MODEL_REFS.length);
      setCopyState("idle");
    }, ROTATION_INTERVAL_MS);

    return () => window.clearInterval(interval);
  }, []);

  useEffect(() => {
    if (copyState !== "copied") return undefined;
    const timeout = window.setTimeout(() => setCopyState("idle"), 1800);
    return () => window.clearTimeout(timeout);
  }, [copyState]);

  const copy = async () => {
    try {
      await copyText(command);
      setCopyState("copied");
    } catch {
      setCopyState("failed");
    }
  };

  return (
    <div className="command-bar rolling-pull-command" aria-label={`Example command: ${command}`}>
      <span className="sr-only">Example pull command</span>
      <code>
        <span>tako pull&nbsp;</span>
        <span className="rolling-pull-command__window" aria-hidden="true">
          <span className="rolling-pull-command__model" key={model}>{model}</span>
        </span>
      </code>
      <button type="button" onClick={copy} aria-live="polite">
        {copyState === "copied" ? "Copied" : copyState === "failed" ? "Copy failed" : "Copy"}
      </button>
    </div>
  );
}
