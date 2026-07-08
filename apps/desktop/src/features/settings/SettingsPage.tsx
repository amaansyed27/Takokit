import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Section } from "../../components/ui/Section";
import { Tooltip } from "../../components/ui/Tooltip";

export function SettingsPage({ runtime }: RouteComponentProps) {
  return (
    <section className="page">
      <header className="page__header">
        <h1>Settings</h1>
        <p>Local paths and safe defaults.</p>
      </header>

      <Section title="Storage">
        <div className="settings-group">
          <p>Files stay under the Takokit storage root.</p>
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
          <p>No hidden remote calls.</p>
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

      <Section title="Controls" description="Safe by default.">
        <div className="settings-group">
          <div className="setting-switch-grid">
            <div className="setting-toggle-row">
              <div>
                <strong>No hidden cloud calls</strong>
                <span>Remote providers disabled.</span>
              </div>
              <span className="switch" aria-hidden="true" />
            </div>
            <div className="setting-toggle-row">
              <div>
                <strong>Consent gate for cloning</strong>
                <span>Required before clone or train.</span>
              </div>
              <span className="switch" aria-hidden="true" />
            </div>
            <div className="setting-toggle-row">
              <div>
                <strong>Auto-download models</strong>
                <span>User initiated only.</span>
              </div>
              <span className="switch is-off" aria-hidden="true" />
            </div>
          </div>
        </div>
      </Section>
    </section>
  );
}
