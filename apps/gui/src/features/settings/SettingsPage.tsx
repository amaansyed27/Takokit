import {
  Check,
  Database,
  HardDrive,
  Moon,
  Server,
  ShieldCheck,
  Sun
} from "lucide-react";
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
        description="A small set of local preferences. Runtime maintenance and storage management stay in their dedicated pages."
      />

      <div className="tk-settings-layout">
        <div className="tk-settings-main">
          <section className="tk-settings-group">
            <header className="tk-settings-group__header">
              <div>
                <span>Appearance</span>
                <p>Choose how Takokit looks on this device.</p>
              </div>
            </header>

            <div className="tk-theme-choices">
              <ThemeOption
                theme="dark"
                active={theme === "dark"}
                title="Dark"
                description="Graphite canvas with soft grey surfaces."
                icon={<Moon size={18} />}
                onSelect={setTheme}
              />
              <ThemeOption
                theme="light"
                active={theme === "light"}
                title="Light"
                description="Warm off-white paper-like surfaces."
                icon={<Sun size={18} />}
                onSelect={setTheme}
              />
            </div>
          </section>

          <section className="tk-settings-group">
            <header className="tk-settings-group__header">
              <div>
                <span>Data & workspace</span>
                <p>Takokit keeps reusable runtime data separate from project-specific outputs.</p>
              </div>
            </header>

            <div className="tk-settings-list">
              <SettingRow
                icon={<Database size={17} strokeWidth={1.7} />}
                title="Active workspace"
                description="Sessions, transcripts, and generated outputs for the current project."
                value={runtime.workspacePath}
              />
              <SettingRow
                icon={<HardDrive size={17} strokeWidth={1.7} />}
                title="Takokit storage"
                description="Models, runners, adapters, reusable voices, cache, and logs."
                value={runtime.storagePath}
                action="Open Storage"
                onAction={() => onNavigate("storage")}
              />
            </div>
          </section>

          <section className="tk-settings-group">
            <header className="tk-settings-group__header">
              <div>
                <span>Safety & behavior</span>
                <p>These rules are enforced by Takokit rather than exposed as cosmetic switches.</p>
              </div>
            </header>

            <div className="tk-settings-policy-list">
              <Policy
                title="Local-first execution"
                description="Workflows run through the local model and runner you selected. Takokit does not silently switch to a cloud provider."
              />
              <Policy
                title="Explicit voice permission"
                description="Creating or converting a voice requires an affirmative ownership or permission confirmation."
              />
              <Policy
                title="User-initiated downloads"
                description="Models and runners are installed only after you explicitly start a pull, install, or repair action."
              />
              <Policy
                title="Workspace isolation"
                description="Project outputs remain in the selected workspace while reusable models, runners, and voices stay in the global Takokit store."
              />
            </div>
          </section>
        </div>

        <aside className="tk-settings-device" aria-label="This device">
          <div className="tk-settings-device__heading">
            <span className={runtime.server.status === "online" ? "tk-settings-device__icon is-online" : "tk-settings-device__icon"}>
              <Server size={18} strokeWidth={1.7} />
            </span>
            <div>
              <strong>This device</strong>
              <span>{runtime.server.status === "online" ? "Local runtime connected" : "Runtime offline"}</span>
            </div>
          </div>

          <dl className="tk-settings-device__facts">
            <DeviceFact label="Status" value={runtime.server.status === "online" ? "Connected locally" : "Offline"} />
            <DeviceFact label="Local API" value={runtime.server.url} />
            <DeviceFact label="Build" value={runtime.buildId} mono />
          </dl>

          <div className="tk-settings-device__actions">
            <button type="button" onClick={() => onNavigate("diagnostics")}>Diagnostics →</button>
            <button type="button" onClick={() => onNavigate("storage")}>Storage →</button>
          </div>

          <div className="tk-settings-device__note">
            <ShieldCheck size={16} strokeWidth={1.7} />
            <p>CLI, TUI, and GUI all use this same local runtime and storage root.</p>
          </div>
        </aside>
      </div>
    </section>
  );
}

function ThemeOption({
  theme,
  active,
  title,
  description,
  icon,
  onSelect
}: {
  theme: TakokitTheme;
  active: boolean;
  title: string;
  description: string;
  icon: JSX.Element;
  onSelect: (theme: TakokitTheme) => void;
}) {
  return (
    <button
      className={active ? "tk-theme-choice is-active" : "tk-theme-choice"}
      type="button"
      onClick={() => onSelect(theme)}
    >
      <span className={`tk-theme-choice__preview is-${theme}`}>
        <span />
        <span />
        <span />
      </span>
      <span className="tk-theme-choice__copy">
        <span>{icon}<strong>{title}</strong></span>
        <small>{description}</small>
      </span>
      {active ? <span className="tk-theme-choice__check"><Check size={14} strokeWidth={2.2} /></span> : null}
    </button>
  );
}

function SettingRow({
  icon,
  title,
  description,
  value,
  action,
  onAction
}: {
  icon: JSX.Element;
  title: string;
  description: string;
  value: string;
  action?: string;
  onAction?: () => void;
}) {
  return (
    <div className="tk-settings-row">
      <span className="tk-settings-row__icon">{icon}</span>
      <div className="tk-settings-row__copy">
        <strong>{title}</strong>
        <span>{description}</span>
        <code title={value}>{value}</code>
      </div>
      {action && onAction ? <button type="button" onClick={onAction}>{action} →</button> : null}
    </div>
  );
}

function DeviceFact({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div>
      <dt>{label}</dt>
      <dd className={mono ? "is-mono" : ""} title={value}>{value}</dd>
    </div>
  );
}

function Policy({ title, description }: { title: string; description: string }) {
  return (
    <div className="tk-settings-policy">
      <span><ShieldCheck size={16} strokeWidth={1.7} /></span>
      <div>
        <strong>{title}</strong>
        <p>{description}</p>
      </div>
    </div>
  );
}
