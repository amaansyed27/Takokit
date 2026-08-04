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
  const origin = typeof window === "undefined"
    ? "https://takokit-library.vercel.app"
    : window.location.origin;

  return (
    <section className="platform-install" aria-labelledby="platform-install-heading">
      <div className="platform-install-head">
        <div>
          <p className="eyebrow">Detected platform</p>
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
      <p className="platform-install-label">{details.label} · {details.shell}</p>
      <CommandBar label={`${details.label} install command`}>
        {installCommand(platform, origin)}
      </CommandBar>
      <p className="platform-install-note">{details.note}</p>
    </section>
  );
}
