import { useState } from "react";
import { RouteLink } from "../../app/router";
import { useMediaQuery, useReducedMotion, useScrollProgress } from "../../hooks/useScrollProgress";

const CAPABILITIES = [
  {
    task: "speech",
    number: "01",
    title: "SPEAK",
    category: "TEXT TO SPEECH",
    statement: "TURN TEXT INTO LOCAL AUDIO.",
    description: "Pull a supported voice model, choose a voice, and write the result directly into your active project session.",
    command: "tako speak \"Hello from Takokit\" --model kokoro",
  },
  {
    task: "transcription",
    number: "02",
    title: "TRANSCRIBE",
    category: "SPEECH TO TEXT",
    statement: "TURN RECORDINGS INTO TEXT.",
    description: "Use Whisper and other transcription families through the same model, runner, session, and output system.",
    command: "tako transcribe ./sample.wav --model whisper-tiny",
  },
  {
    task: "cloning",
    number: "03",
    title: "CLONE",
    category: "VOICE CLONING",
    statement: "CREATE A CONSENTED VOICE PROFILE.",
    description: "Register owned reference audio, keep it local, and reuse the voice profile across compatible models.",
    command: "tako clone ./reference.wav --name \"My Voice\" --model chatterbox --consent",
  },
  {
    task: "conversion",
    number: "04",
    title: "CONVERT",
    category: "VOICE CONVERSION",
    statement: "RESHAPE A RECORDING LOCALLY.",
    description: "Run OpenVoice or RVC workflows with explicit source, target, model, and consent controls.",
    command: "tako convert ./source.wav --target-voice ./target.wav --model openvoice --consent",
  },
];

export function CapabilityPinwheel() {
  const reducedMotion = useReducedMotion();
  const narrowLayout = useMediaQuery("(max-width: 920px)");
  const staticLayout = reducedMotion || narrowLayout;
  const [activeIndex, setActiveIndex] = useState(0);
  const sectionRef = useScrollProgress((progress) => {
    const bounded = Math.min(0.9999, Math.max(0, progress));
    const nextIndex = Math.min(CAPABILITIES.length - 1, Math.floor(bounded * CAPABILITIES.length));
    setActiveIndex((current) => (current === nextIndex ? current : nextIndex));
  }, staticLayout);

  function jumpToCapability(index) {
    if (staticLayout || !sectionRef.current) {
      setActiveIndex(index);
      return;
    }
    const section = sectionRef.current;
    const top = window.scrollY + section.getBoundingClientRect().top;
    const range = Math.max(section.offsetHeight - window.innerHeight, 1);
    window.scrollTo({ top: top + range * ((index + 0.12) / CAPABILITIES.length), behavior: "smooth" });
  }

  const active = CAPABILITIES[activeIndex];
  const wheelTurn = `${-activeIndex * 90}deg`;

  return (
    <section
      className={`tk-wheel ${staticLayout ? "is-static" : ""}`}
      id="workflows"
      ref={sectionRef}
      aria-labelledby="tk-wheel-title"
      style={{ "--tk-wheel-turn": wheelTurn }}
    >
      <div className="tk-wheel__stage shell">
        <header className="tk-section-bar tk-section-bar--light">
          <span>02 / ONE RUNTIME</span>
          <span>SPEAK / TRANSCRIBE / CLONE / CONVERT</span>
        </header>

        <div className="tk-wheel__orbit" aria-hidden="true">
          <div className="tk-wheel__rotor">
            {CAPABILITIES.map((capability, index) => (
              <div
                className={`tk-wheel__spoke ${activeIndex === index ? "is-active" : ""}`}
                key={capability.task}
                style={{
                  "--spoke-angle": `${index * 90}deg`,
                  "--label-turn": `${(activeIndex - index) * 90}deg`,
                }}
              >
                <div>
                  <span>{capability.number}</span>
                  <strong>{capability.title}</strong>
                </div>
              </div>
            ))}
            <div className="tk-wheel__core">
              <img src="/brand/takokit-mark.svg" alt="" />
            </div>
          </div>
        </div>

        <div className="tk-wheel__copy" aria-live="polite">
          <p className="tk-wheel__index">{active.number} / 04</p>
          <p className="tk-kicker">{active.category}</p>
          <h2 id="tk-wheel-title">{active.title}</h2>
          <p className="tk-wheel__statement">{active.statement}</p>
          <p className="tk-wheel__description">{active.description}</p>
          <code>{active.command}</code>
          <RouteLink href={`/models?task=${active.task}`} className="tk-text-link">
            Browse compatible models →
          </RouteLink>
        </div>

        <div className="tk-wheel__selector" role="tablist" aria-label="Choose a Takokit workflow">
          {CAPABILITIES.map((capability, index) => (
            <button
              aria-selected={activeIndex === index}
              className={activeIndex === index ? "is-active" : ""}
              key={capability.task}
              onClick={() => jumpToCapability(index)}
              role="tab"
              type="button"
            >
              <span>{capability.number}</span>
              <strong>{capability.title}</strong>
            </button>
          ))}
        </div>
      </div>
    </section>
  );
}
