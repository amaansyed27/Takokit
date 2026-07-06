import { useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Section } from "../../components/ui/Section";
import { Table, TableRow } from "../../components/ui/Table";
import { Tooltip } from "../../components/ui/Tooltip";

export function ModelsPage({ runtime }: RouteComponentProps) {
  const [query, setQuery] = useState("");
  const models = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return runtime.models;
    return runtime.models.filter((model) =>
      [model.name, model.purpose, model.runtime, model.status, model.license].some((value) => value.toLowerCase().includes(needle))
    );
  }, [query, runtime.models]);

  return (
    <section className="page">
      <header className="page__header">
        <h1>Models</h1>
        <p>Installed and available registry entries. Real inference only starts after a runner is wired.</p>
      </header>

      <Section title="Registry">
        <input
          className="search-input"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Filter by model, runtime, license, or status..."
          aria-label="Filter models"
        />
        <Table columns={["Model", "Purpose", "Runtime", "Status", "License"]} ariaLabel="Models">
          {models.map((model) => (
            <TableRow key={model.id}>
              <strong>{model.name}</strong>
              <span>{model.purpose}</span>
              <Tooltip content={`${model.backend} backend, ${model.params} params`}>
                <span>{model.runtime}</span>
              </Tooltip>
              <Badge tone={model.status === "installed" ? "success" : model.status === "available" ? "neutral" : "warning"}>
                {model.status}
              </Badge>
              <Tooltip content="Verify model license before commercial use.">
                <span>{model.license}</span>
              </Tooltip>
            </TableRow>
          ))}
        </Table>
      </Section>
    </section>
  );
}

