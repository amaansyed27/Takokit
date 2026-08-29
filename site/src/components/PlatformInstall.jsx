import { useState } from "react";
import { CommandBar } from "./CommandBar";
import {
  detectPlatform,
  installCommand,
  PLATFORM_DETAILS,
  PLATFORM_ORDER,
} from "../lib/platform";

export function PlatformInstall({ heading = "Install Takokit" }) {
  const [platform, setPlatform] = useState(() => detectPlatform());
  const details = PLATFORM_DETAILS[platform];
  const command = installCommand(platform);

  return (
    <section className="platform-install" aria-labelledby="platform-install-heading">
      <div className="platform-install-head">
        <div>
          <p className="eyebrow">Choose platform</p>
          <h2 id="platform-install-heading">{heading}</h2>
        </div>
        <div className="platform-switcher" role="tablist" aria-label="Choose operating system">
          {PLATFORM_ORDER.map((key) => (
            <button
              type="button"
              role="tab"
              aria-selected={platform === key}
              className={platform === key ? "is-active" : ""}
              onClick={() => setPlatform(key)}
              key={key}
            >
              {PLATFORM_DETAILS[key].label}
            </button>
          ))}
        </div>
      </div>

      <div role="tabpanel" className="platform-panel">
        <p className="platform-install-label">{details.label} · {details.available ? details.shell : "Coming later"}</p>
        {command ? (
          <CommandBar label="Windows PowerShell install command">{command}</CommandBar>
        ) : (
          <p className="platform-unavailable" role="status">{details.label} packages are not available yet.</p>
        )}
        <p className="platform-install-note">{details.note}</p>
      </div>
    </section>
  );
}
