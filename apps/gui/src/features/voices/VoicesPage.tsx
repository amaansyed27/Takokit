import { useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
import { Table, TableRow } from "../../components/ui/Table";
import { Tooltip } from "../../components/ui/Tooltip";

export function VoicesPage({ runtime }: RouteComponentProps) {
  const [selectedVoice, setSelectedVoice] = useState(runtime.voices[0]?.id ?? "");
  const selected = runtime.voices.find((voice) => voice.id === selectedVoice) ?? runtime.voices[0];

  return (
    <section className="page">
      <header className="page__header">
        <h1>Voices</h1>
        <p>Local profiles and consent gates.</p>
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
        <div className="voice-workbench">
          <div>
            <strong>{selected?.name ?? "No voice selected"}</strong>
            <p>{selected?.label ?? "Choose a voice profile from the list."}</p>
            <p>Custom voices appear after consent and runner wiring.</p>
            <div className="voice-meta-grid">
              <div className="voice-meta">
                <span>Source</span>
                <strong>{selected?.consent === "not required" ? "Bundled test profile" : "User supplied"}</strong>
              </div>
              <div className="voice-meta">
                <span>Model</span>
                <strong>{selected?.model ?? "none"}</strong>
              </div>
              <div className="voice-meta">
                <span>Boundary</span>
                <strong>Local only</strong>
              </div>
            </div>
          </div>
          <div className="voice-wave" aria-hidden="true">
            <span />
            <span />
            <span />
            <span />
            <span />
            <span />
          </div>
          <div>
            <Tooltip content="Voice cloning is disabled until consent and runner wiring are complete.">
              <Button disabled>Add cloned voice</Button>
            </Tooltip>
          </div>
        </div>
      </Section>
    </section>
  );
}
