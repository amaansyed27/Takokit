import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Section } from "../../components/ui/Section";
import { Tooltip } from "../../components/ui/Tooltip";

export function SettingsPage({ runtime }: RouteComponentProps) {
  return (
    <section className="page">
      <header className="page__header">
        <h1>Settings</h1>
        <p>Local paths and runtime defaults stay explicit and user-controlled.</p>
      </header>

      <Section title="Storage">
        <div className="settings-group">
          <div className="settings-list">
            <div className="settings-row"><span>Storage root</span><code>{runtime.storagePath}</code></div>
            <div className="settings-row"><span>Models</span><code>{runtime.storagePath}/models</code></div>
            <div className="settings-row"><span>Voices</span><code>{runtime.storagePath}/voices</code></div>
            <div className="settings-row"><span>Outputs</span><code>{runtime.storagePath}/outputs</code></div>
          </div>
        </div>
      </Section>

      <Section title="Runtime">
        <div className="settings-group">
          <div className="settings-row"><span>Theme</span><Badge>Paper</Badge></div>
          <div className="settings-row"><span>Runtime mode</span><Badge tone="success">Local</Badge></div>
          <div className="settings-row">
            <span>Safety and consent</span>
            <Tooltip content="Voice cloning consent gates are planned before runner wiring.">
              <Badge tone="warning">Required for cloning</Badge>
            </Tooltip>
          </div>
        </div>
      </Section>
    </section>
  );
}

