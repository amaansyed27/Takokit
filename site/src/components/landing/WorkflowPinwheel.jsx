import { useState } from "react";
import { RouteLink } from "../../app/router";
import { useMediaQuery, useReducedMotion, useScrollProgress } from "../../hooks/useScrollProgress";

const WORKFLOWS = [
  {
    task: "speech",
    title: "Speak",
    category: "Text to speech",
    statement: "Turn text into local audio.",
    description: "Choose a supported model and voice, then write the result into the active local session.",
    command: "tako speak \"Hello from Takokit\" --model kokoro",
  },
  {
    task: "transcription",
    title: "Transcribe",
    category: "Speech to text",
    statement: "Turn recordings into text.",
    description: "Run Whisper and other transcription models through the same registry, runner, and output system.",
    command: "tako transcribe ./sample.wav --model whisper-tiny",
  },
  {
    task: "cloning",
    title: "Clone",
    category: "Voice cloning",
    statement: "Create a consented voice profile.",
    description: "Register owned reference audio locally and reuse the resulting voice profile with compatible models.",
    command: "tako clone ./reference.wav --name \"My Voice\" --model chatterbox --consent",
  },
  {
    task: "conversion",
    title: "Convert",
    category: "Voice conversion",
    statement: "Reshape a recording locally.",
    description: "Run OpenVoice or RVC workflows with explicit source, target, model, and consent controls.",
    command: "tako convert ./source.wav --target-voice ./target.wav --model openvoice --consent",
  },
];

export function WorkflowPinwheel() {
  const reducedMotion = useReducedMotion();
  const compactLayout = useMediaQuery("(max-width: 920px)");
  const staticLayout = reducedMotion || compactLayout;
  const [activeIndex, setActiveIndex] = useState(0);
  const sectionRef = useScrollProgress((progress) => {
    const bounded = Math.min(0.9999, Math.max(0, progress));
    const nextIndex = Math.min(WORKFLOWS.length - 1, Math.floor(bounded * WORKFLOWS.length));
    setActiveIndex((current) => (current === nextIndex ? current : nextIndex));
  }, staticLayout);

  function selectWorkflow(index) {
    if (staticLayout || !sectionRef.current) {
      setActiveIndex(index);
      return;
    }
    const section = sectionRef.current;
    const sectionTop = window.scrollY + section.getBoundingClientRect().top;
    const range = Math.max(section.offsetHeight - window.innerHeight, 1);
    window.scrollTo({
      top: sectionTop + range * ((index + 0.15) / WORKFLOWS.length),
      behavior: "smooth",
    });
  }

  const active = WORKFLOWS[activeIndex];
  const turn = `${activeIndex * -90}deg`;
  const counterTurn = `${activeIndex * 90}deg`;

  return (
    <section
      className={`workflow-pinwheel ${staticLayout ? "is-static" : ""}`}
      id="workflows"
      ref={sectionRef}
      style={{ "--wheel-turn": turn, "--wheel-counter-turn": counterTurn }}
      aria-labelledby="workflow-pinwheel-title"
    >
      <div className="workflow-pinwheel__stage landing-shell">
        <div className="workflow-pinwheel__visual" aria-hidden="true">
          <div className="workflow-pinwheel__wheel">
            {WORKFLOWS.map((workflow, index) => (
              <div
                className={`workflow-pinwheel__item ${activeIndex === index ? "is-active" : ""}`}
                key={workflow.task}
                style={{ "--item-angle": `${index * 90}deg`, "--item-counter-angle": `${index * -90}deg` }}
              >
                <div>
                  <span>{String(index + 1).padStart(2, "0")}</span>
                  <strong>{workflow.title}</strong>
                </div>
              </div>
            ))}
            <div className="workflow-pinwheel__core">
              <img src="/brand/takokit-mark.svg" alt="" />
            </div>
          </div>
        </div>

        <div className="workflow-pinwheel__copy" aria-live="polite">
          <p className="landing-kicker">One runtime / four workflows</p>
          <span className="workflow-pinwheel__index">{String(activeIndex + 1).padStart(2, "0")} / 04</span>
          <h2 id="workflow-pinwheel-title">{active.title}</h2>
          <strong>{active.statement}</strong>
          <p>{active.description}</p>
          <code>{active.command}</code>
          <RouteLink href={`/models?task=${active.task}`} className="landing-text-link">Browse compatible models →</RouteLink>
        </div>

        <div className="workflow-pinwheel__tabs" role="tablist" aria-label="Choose a Takokit workflow">
          {WORKFLOWS.map((workflow, index) => (
            <button
              type="button"
              role="tab"
              aria-selected={activeIndex === index}
              className={activeIndex === index ? "is-active" : ""}
              key={workflow.task}
              onClick={() => selectWorkflow(index)}
            >
              <span>{String(index + 1).padStart(2, "0")}</span>
              <strong>{workflow.title}</strong>
              <small>{workflow.category}</small>
            </button>
          ))}
        </div>
      </div>
    </section>
  );
}
