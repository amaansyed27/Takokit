import { useId, useState } from "react";

function Check({ label, checked, onChange }) {
  return (
    <label className="check-filter">
      <input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} />
      <span>{label}</span>
    </label>
  );
}

export function AdvancedFilters({ filters, runners, onChange, onReset }) {
  const [open, setOpen] = useState(false);
  const id = useId();
  const patch = (key, value) => onChange({ ...filters, [key]: value });
  const activeCount = [
    filters.cpuFriendly,
    filters.gpuSupported,
    filters.gpuRequired,
    filters.maxVram,
    filters.maxSize,
    filters.status,
    filters.commercial,
    filters.platform,
    filters.runner,
  ].filter(Boolean).length;

  return (
    <div className="advanced-filter">
      <button
        className="filter-toggle"
        type="button"
        aria-expanded={open}
        aria-controls={id}
        onClick={() => setOpen((value) => !value)}
      >
        Filters{activeCount ? ` (${activeCount})` : ""}
      </button>
      {open && (
        <div id={id} className="filter-panel">
          <fieldset>
            <legend>Hardware</legend>
            <Check label="CPU friendly" checked={filters.cpuFriendly} onChange={(value) => patch("cpuFriendly", value)} />
            <Check label="GPU supported" checked={filters.gpuSupported} onChange={(value) => patch("gpuSupported", value)} />
            <Check label="GPU required" checked={filters.gpuRequired} onChange={(value) => patch("gpuRequired", value)} />
            <label>Maximum VRAM
              <select value={filters.maxVram} onChange={(event) => patch("maxVram", event.target.value)}>
                <option value="">Any</option><option value="2">2 GB</option><option value="4">4 GB</option>
                <option value="6">6 GB</option><option value="8">8 GB</option><option value="12">12 GB</option>
                <option value="24">24 GB</option><option value="40">40 GB</option>
              </select>
            </label>
            <label>Maximum download size
              <select value={filters.maxSize} onChange={(event) => patch("maxSize", event.target.value)}>
                <option value="">Any</option><option value="100">100 MB</option><option value="250">250 MB</option>
                <option value="500">500 MB</option><option value="1000">1 GB</option><option value="3000">3 GB</option>
              </select>
            </label>
          </fieldset>
          <fieldset>
            <legend>Status and licence</legend>
            <label>Verification
              <select value={filters.status} onChange={(event) => patch("status", event.target.value)}>
                <option value="">Any</option>
                <option value="verified">Verified</option>
                <option value="executable">Experimental / executable path</option>
                <option value="metadata-only">Metadata only</option>
                <option value="hardware-blocked">Hardware blocked</option>
              </select>
            </label>
            <label>Commercial use
              <select value={filters.commercial} onChange={(event) => patch("commercial", event.target.value)}>
                <option value="">Any</option><option value="yes">Declared compatible</option>
                <option value="no">Non-commercial</option><option value="unknown">Not declared / review required</option>
              </select>
            </label>
          </fieldset>
          <fieldset>
            <legend>Platforms</legend>
            {["Windows", "Linux", "macOS"].map((platform) => (
              <Check
                key={platform}
                label={platform}
                checked={filters.platform === platform}
                onChange={(checked) => patch("platform", checked ? platform : "")}
              />
            ))}
          </fieldset>
          <details>
            <summary>Advanced runtime filters</summary>
            <label>Runner
              <select value={filters.runner} onChange={(event) => patch("runner", event.target.value)}>
                <option value="">Any runner</option>
                {runners.map((runner) => <option key={runner} value={runner}>{runner}</option>)}
              </select>
            </label>
          </details>
          <button className="text-button" type="button" onClick={onReset}>Reset filters</button>
        </div>
      )}
    </div>
  );
}
