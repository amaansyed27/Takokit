import { useEffect, useMemo, useState } from "react";
import { navigate } from "../app/router";
import { AdvancedFilters } from "../components/AdvancedFilters";
import { EmptyState, ErrorState, LoadingState } from "../components/States";
import { ModelRow } from "../components/ModelRow";
import { TaskFilter } from "../components/TaskFilter";
import { useRegistry } from "../hooks/useRegistry";
import {
  DEFAULT_FILTERS,
  filterModels,
  filtersFromSearch,
  searchFromFilters,
} from "../models/filtering";

export function ModelsPage({ location }) {
  const { status, registry, error, retry } = useRegistry();
  const initial = useMemo(() => filtersFromSearch(location.query), [location.search]);
  const [filters, setFilters] = useState(initial);

  useEffect(() => setFilters(initial), [initial]);

  const update = (next) => {
    setFilters(next);
    navigate(`/models${searchFromFilters(next)}`, { replace: true });
  };

  const models = status === "ready" ? filterModels(registry.models, filters) : [];
  const runners = status === "ready"
    ? [...new Set(registry.models.map((model) => model.release.runner))].sort()
    : [];

  return (
    <main className="shell page models-page">
      <header className="compact-page-head">
        <p className="eyebrow">Model library</p>
        <h1>Models</h1>
        <p>Find a model by task, hardware, size, verification evidence, or licence status.</p>
      </header>

      <div className="model-controls">
        <label className="search-field">
          <span>Search models</span>
          <input
            type="search"
            value={filters.query}
            placeholder="Name, task, language, hardware…"
            onChange={(event) => update({ ...filters, query: event.target.value })}
          />
        </label>
        <TaskFilter value={filters.task} onChange={(task) => update({ ...filters, task })} />
        <div className="control-row">
          <AdvancedFilters
            filters={filters}
            runners={runners}
            onChange={update}
            onReset={() => update({ ...DEFAULT_FILTERS, query: filters.query, sort: filters.sort })}
          />
          <label className="sort-control">Sort
            <select value={filters.sort} onChange={(event) => update({ ...filters, sort: event.target.value })}>
              <option value="recommended">Recommended</option>
              <option value="name">Name</option>
              <option value="smallest">Smallest download</option>
              <option value="hardware">Lowest hardware requirement</option>
              <option value="verified">Recently verified</option>
            </select>
          </label>
        </div>
      </div>

      <p className="result-count" aria-live="polite">
        {status === "ready" ? `${models.length} model ${models.length === 1 ? "family" : "families"}` : "Loading model results"}
      </p>

      {status === "loading" && <LoadingState />}
      {status === "error" && <ErrorState error={error} onRetry={retry} />}
      {status === "ready" && !models.length && <EmptyState />}
      {status === "ready" && models.length > 0 && (
        <div className="model-results">{models.map((model) => <ModelRow key={model.name} model={model} />)}</div>
      )}
    </main>
  );
}
