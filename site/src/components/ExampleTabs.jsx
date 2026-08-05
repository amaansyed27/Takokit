import { useState } from "react";
import { CommandBar } from "./CommandBar";

export function ExampleTabs({ examples }) {
  const names = Object.keys(examples);
  const [active, setActive] = useState(names[0]);
  return (
    <div className="example-tabs">
      <div className="tab-list" role="tablist" aria-label="Integration examples">
        {names.map((name) => (
          <button
            key={name}
            type="button"
            role="tab"
            aria-selected={active === name}
            className={active === name ? "is-active" : ""}
            onClick={() => setActive(name)}
          >
            {name}
          </button>
        ))}
      </div>
      <div role="tabpanel">
        <p className="example-note">
          {active === "Python" || active === "JavaScript"
            ? "Uses a standard HTTP client. Takokit does not currently publish an official SDK."
            : "This example matches Takokit's current local interface."}
        </p>
        <CommandBar label={`${active} example`}>{examples[active]}</CommandBar>
      </div>
    </div>
  );
}
