import { useEffect, useState } from "react";
import { Download, RefreshCw } from "lucide-react";
import {
  applyUpdate,
  checkForUpdates,
  configureUpdates,
  getUpdateStatus,
  type UpdateStatus
} from "../../lib/update";

export function UpdateSettings() {
  const [update, setUpdate] = useState<UpdateStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
  }, []);

  async function refresh() {
    try {
      setUpdate(await getUpdateStatus());
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Could not load update status.");
    }
  }

  async function run(action: () => Promise<unknown>, success?: string) {
    setBusy(true);
    setMessage(null);
    try {
      await action();
      if (success) setMessage(success);
      await refresh();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Update action failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="tk-settings-group tk-update-settings">
      <header className="tk-settings-group__header">
        <div>
          <span>Application updates</span>
          <p>Signed runtime updates are separate from model and library updates.</p>
        </div>
        <button
          className="tk-update-check"
          type="button"
          disabled={busy || !update?.manifest_source}
          onClick={() => run(checkForUpdates, "Update check completed.")}
        >
          <RefreshCw size={15} /> Check
        </button>
      </header>

      <div className="tk-update-summary">
        <Fact label="Current" value={update?.current_version ?? "Loading…"} />
        <Fact label="Available" value={update?.available_version ?? "None checked"} />
        <Fact label="Downloaded" value={update?.downloaded_version ?? "No"} />
        <Fact label="Distribution" value={update?.distribution_mode ?? "Unknown"} />
      </div>

      <div className="tk-update-controls">
        <label className="tk-update-channel">
          <span>Release channel</span>
          <select
            value={update?.channel ?? "stable"}
            disabled={busy || !update}
            onChange={(event) => {
              const channel = event.target.value as "stable" | "preview";
              void run(() => configureUpdates({ channel }));
            }}
          >
            <option value="stable">Stable</option>
            <option value="preview">Preview</option>
          </select>
        </label>

        <Toggle
          title="Automatic checks"
          description="Check the signed manifest opportunistically, at most once per day."
          checked={update?.automatic_checks ?? true}
          disabled={busy || !update}
          onChange={(automatic_checks) => run(() => configureUpdates({ automatic_checks }))}
        />
        <Toggle
          title="Automatic download"
          description="Opt in to verified background download. Installation always stays manual."
          checked={update?.automatic_download ?? false}
          disabled={busy || !update}
          onChange={(automatic_download) => run(() => configureUpdates({ automatic_download }))}
        />
      </div>

      <div className="tk-update-install-row">
        <div>
          <strong>
            {update?.available_version
              ? `Takokit ${update.available_version} is available`
              : "No verified update is ready to install"}
          </strong>
          <span>
            Installation is refused while inference, conversion, model pulls, adapter installs, or RVC training are active.
          </span>
        </div>
        <button
          type="button"
          disabled={busy || !update?.available_version || update.distribution_mode !== "installed"}
          onClick={() =>
            run(
              applyUpdate,
              "Update installation requested. Takokit may briefly close while the verified replacement is applied."
            )
          }
        >
          <Download size={15} /> Install update
        </button>
      </div>

      {message || update?.last_error ? (
        <p className="tk-update-message">{message ?? update?.last_error}</p>
      ) : null}
    </section>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function Toggle({
  title,
  description,
  checked,
  disabled,
  onChange
}: {
  title: string;
  description: string;
  checked: boolean;
  disabled: boolean;
  onChange: (checked: boolean) => Promise<unknown>;
}) {
  return (
    <label className="tk-update-toggle">
      <span>
        <strong>{title}</strong>
        <small>{description}</small>
      </span>
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(event) => void onChange(event.target.checked)}
      />
    </label>
  );
}
