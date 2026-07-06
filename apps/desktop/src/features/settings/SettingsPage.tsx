import { SectionHeader } from "../../components/SectionHeader";
import type { RuntimeSnapshot } from "../../lib/types";

type SettingsPageProps = {
  runtime: RuntimeSnapshot;
};

export function SettingsPage({ runtime }: SettingsPageProps) {
  return (
    <section className="page-flow">
      <SectionHeader title="Settings" description="Local paths and runtime defaults are explicit and user-controlled." />
      <div className="settings-list">
        <div><span>Storage root</span><code>{runtime.storagePath}</code></div>
        <div><span>Models</span><code>{runtime.storagePath}/models</code></div>
        <div><span>Voices</span><code>{runtime.storagePath}/voices</code></div>
        <div><span>Outputs</span><code>{runtime.storagePath}/outputs</code></div>
        <div><span>Config</span><code>{runtime.storagePath}/config.toml</code></div>
      </div>
    </section>
  );
}

