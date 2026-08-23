import { Check, HardDrive, Moon, ShieldCheck, Sun } from "lucide-react";
import type { RouteComponentProps } from "../../app/routes";
import { ProductPageHeader } from "../../components/ui/ProductPageHeader";
import { useTheme, type TakokitTheme } from "../../hooks/useTheme";

export function SettingsPage({ runtime, onNavigate }: RouteComponentProps) {
  const { theme, setTheme } = useTheme();

  return (
    <section className="tk-page tk-settings-page">
      <ProductPageHeader
        eyebrow="Preferences"
        title="Settings"
        description="Small, explicit preferences for the local GUI. Runtime lifecycle and storage management live in their dedicated pages."
      />

      <section className="tk-settings-section">
        <div className="tk-settings-section__heading"><div><h2>Appearance</h2><p>Choose the interface that is easiest on your eyes. This preference stays on this browser.</p></div></div>
        <div className="tk-theme-options">
          <ThemeOption theme="dark" active={theme === "dark"} title="Dark" description="Graphite canvas with soft grey surfaces." icon={<Moon size={18} />} onSelect={setTheme} />
          <ThemeOption theme="light" active={theme === "light"} title="Light" description="Warm off-white paper-like surfaces." icon={<Sun size={18} />} onSelect={setTheme} />
        </div>
      </section>

      <section className="tk-settings-section">
        <div className="tk-settings-section__heading"><div><h2>Local runtime</h2><p>The GUI is a client of the same Takokit runtime used by CLI and TUI.</p></div></div>
        <div className="tk-settings-facts">
          <Fact label="Status" value={runtime.server.status === "online" ? "Connected locally" : "Offline"} />
          <Fact label="Workspace" value={runtime.workspacePath} />
          <Fact label="Storage root" value={runtime.storagePath} />
          <Fact label="Local API" value={runtime.server.url} />
          <Fact label="Build" value={runtime.buildId} mono />
        </div>
        <div className="tk-settings-links">
          <button type="button" onClick={() => onNavigate("storage")}><HardDrive size={15} /> Storage</button>
          <button type="button" onClick={() => onNavigate("diagnostics")}>Diagnostics →</button>
        </div>
      </section>

      <section className="tk-settings-section">
        <div className="tk-settings-section__heading"><div><h2>Safety and behavior</h2><p>These are enforced product rules, not decorative toggles.</p></div></div>
        <div className="tk-policy-list">
          <Policy title="Local-first execution" description="Takokit workflows execute through the selected local models and runners. The GUI does not silently switch to a cloud provider." />
          <Policy title="Explicit voice consent" description="Creating or converting a voice requires an affirmative ownership or permission gate before the request is accepted." />
          <Policy title="User-initiated model downloads" description="Models and runners are installed only when you explicitly start a pull, install, or repair action." />
          <Policy title="Workspace isolation" description="Sessions and outputs stay under the selected workspace while models, runners, and reusable voices remain in the global Takokit store." />
        </div>
      </section>
    </section>
  );
}

function ThemeOption({ theme, active, title, description, icon, onSelect }: { theme: TakokitTheme; active: boolean; title: string; description: string; icon: JSX.Element; onSelect: (theme: TakokitTheme) => void }) {
  return (
    <button className={active ? "tk-theme-option is-active" : "tk-theme-option"} type="button" onClick={() => onSelect(theme)}>
      <span className="tk-theme-option__preview"><span /><span /><span /></span>
      <span className="tk-theme-option__copy"><span>{icon}<strong>{title}</strong></span><small>{description}</small></span>
      {active ? <Check className="tk-theme-option__check" size={15} /> : null}
    </button>
  );
}

function Fact({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return <div><span>{label}</span><strong className={mono ? "is-mono" : ""} title={value}>{value}</strong></div>;
}

function Policy({ title, description }: { title: string; description: string }) {
  return <div className="tk-policy-row"><span><ShieldCheck size={16} /></span><div><strong>{title}</strong><p>{description}</p></div></div>;
}
