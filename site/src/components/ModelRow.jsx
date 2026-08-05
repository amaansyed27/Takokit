import { RouteLink } from "../app/router";
import { formatBytes } from "../models/registry";
import { CommandBar } from "./CommandBar";
import { HardwareSummary } from "./HardwareSummary";
import { VerificationBadge } from "./VerificationBadge";

export function modelHref(model) {
  return model.release.tag === model.default_tag
    ? `/models/${encodeURIComponent(model.name)}`
    : `/models/${encodeURIComponent(model.name)}/${encodeURIComponent(model.release.tag)}`;
}

export function ModelRow({ model }) {
  return (
    <article className="model-row">
      <RouteLink href={modelHref(model)} className="model-row-link">
        <div className="model-row-main">
          <h2>{model.display_name}</h2>
          <p>{model.shortSummary}</p>
          <div className="model-row-meta">
            <span>{model.taskLabels.join(" · ") || "Task not declared"}</span>
            <HardwareSummary hardware={model.hardware} />
          </div>
        </div>
        <div className="model-row-facts">
          <span>{formatBytes(model.sizeBytes)}</span>
          <VerificationBadge status={model.status} />
        </div>
      </RouteLink>
      <div className="model-row-command">
        <CommandBar compact>tako pull {model.ref}</CommandBar>
      </div>
    </article>
  );
}
