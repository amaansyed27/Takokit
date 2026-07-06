import { useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
import { Table, TableRow } from "../../components/ui/Table";
import { Tooltip } from "../../components/ui/Tooltip";

export function VoicesPage({ runtime }: RouteComponentProps) {
  const [selectedVoice, setSelectedVoice] = useState(runtime.voices[0]?.id ?? "");

  return (
    <section className="page">
      <header className="page__header">
        <h1>Voices</h1>
        <p>Voice profiles stay local. Cloning and training require explicit consent before they are enabled.</p>
      </header>

      <Section title="Profiles">
        <Table columns={["Voice", "Description", "Model", "Consent"]} ariaLabel="Voices" compact>
          {runtime.voices.map((voice) => (
            <TableRow key={voice.id}>
              <button className="button button--ghost" type="button" onClick={() => setSelectedVoice(voice.id)}>
                {voice.name}
              </button>
              <span>{voice.description}</span>
              <span>{voice.model}</span>
              <Badge tone={voice.consent === "not required" ? "success" : "warning"}>{voice.consent}</Badge>
            </TableRow>
          ))}
        </Table>
      </Section>

      <Section title="Selected voice">
        <div className="empty-state">
          <strong>{selectedVoice || "No voice selected"}</strong>
          <p>Custom cloned voices will appear here after consent capture and runner integration are implemented.</p>
          <Tooltip content="Voice cloning is disabled until consent and runner wiring are complete.">
            <Button disabled>Add cloned voice</Button>
          </Tooltip>
        </div>
      </Section>
    </section>
  );
}

