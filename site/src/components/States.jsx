export function LoadingState({ label = "Loading models…" }) {
  return <div className="state-box" role="status"><span className="spinner" />{label}</div>;
}

export function ErrorState({ error, onRetry }) {
  return (
    <div className="state-box state-error" role="alert">
      <h2>Registry could not be loaded</h2>
      <p>{error?.message || "The model registry is unavailable."}</p>
      {Array.isArray(error?.details) && (
        <ul>{error.details.slice(0, 5).map((detail) => <li key={detail}>{detail}</li>)}</ul>
      )}
      <button type="button" onClick={onRetry}>Try again</button>
    </div>
  );
}

export function EmptyState() {
  return (
    <div className="state-box">
      <h2>No matching models</h2>
      <p>Change the search or remove one of the active filters.</p>
    </div>
  );
}
