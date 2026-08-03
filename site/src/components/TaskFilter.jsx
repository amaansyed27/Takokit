const TASKS = [
  ["all", "All"],
  ["speech", "Speech"],
  ["transcription", "Transcription"],
  ["cloning", "Cloning"],
  ["conversion", "Conversion"],
];

export function TaskFilter({ value, onChange }) {
  return (
    <div className="task-filter" role="group" aria-label="Filter models by task">
      {TASKS.map(([id, label]) => (
        <button
          key={id}
          type="button"
          className={value === id ? "is-active" : ""}
          aria-pressed={value === id}
          onClick={() => onChange(id)}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
