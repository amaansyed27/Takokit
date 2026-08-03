import { RouteLink } from "../app/router";
import { formatBytes } from "../models/registry";
import { isRecommended, verificationStatus } from "../models/presentation";
import { hardwareLabel } from "./HardwareSummary";
import { VerificationBadge } from "./VerificationBadge";

export function VariantTable({ model, selected }) {
  return (
    <div className="variant-table" role="table" aria-label={`${model.display_name} variants`}>
      <div className="variant-header" role="row">
        <span>Variant</span><span>Hardware</span><span>Size</span><span>Status</span>
      </div>
      {model.tags.map((release) => {
        const recommended = isRecommended(model, release) || release.tag === model.default_tag;
        const href = release.tag === model.default_tag
          ? `/models/${model.name}`
          : `/models/${model.name}/${release.tag}`;
        return (
          <RouteLink
            href={href}
            key={release.tag}
            role="row"
            className={selected.tag === release.tag ? "variant-row is-selected" : "variant-row"}
          >
            <span><strong>{model.name}:{release.tag}</strong>{recommended && <small>Recommended</small>}</span>
            <span>{hardwareLabel({
              cpu: Boolean(release.hardware?.cpu),
              gpu: Boolean(release.hardware?.gpu),
              gpuRequired: release.hardware?.cpu === false && release.hardware?.gpu === true,
              minVram: release.hardware?.min_vram,
            })}</span>
            <span>{formatBytes(release.size_bytes)}</span>
            <span><VerificationBadge status={verificationStatus(model, release)} /></span>
          </RouteLink>
        );
      })}
    </div>
  );
}
