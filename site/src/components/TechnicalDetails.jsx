export function TechnicalDetails({ release }) {
  const rows = [
    ["Runner", release.runner],
    ["Adapter", release.adapter],
    ["Backend", release.backend],
    ["Exact target", release.target],
    ["Digest", release.digest],
    ["Source repository", release.source?.repository],
    ["Pinned revision", release.source?.revision],
    ["Manifest version", release.version],
  ];
  return (
    <details className="technical-details">
      <summary>Advanced technical details</summary>
      <dl>
        {rows.map(([label, value]) => (
          <div key={label}>
            <dt>{label}</dt>
            <dd>{value || "Not declared"}</dd>
          </div>
        ))}
      </dl>
    </details>
  );
}
