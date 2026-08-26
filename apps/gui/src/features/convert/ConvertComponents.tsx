import { Check, FileAudio, FolderOpen, X } from "lucide-react";
import { LocalAudioPlayer } from "../../components/audio/LocalAudioPlayer";
import { ProductButton } from "../../components/ui/ProductButton";

type ConversionPathCardProps = {
  label: string;
  description: string;
  path: string;
  busy: boolean;
  actionLabel: string;
  onBrowse: () => void;
  onClear: () => void;
  folder?: boolean;
};

export function ConversionPathCard({ label, description, path, busy, actionLabel, onBrowse, onClear, folder = false }: ConversionPathCardProps) {
  return (
    <div className={path ? "tk-convert-path is-selected" : "tk-convert-path"}>
      <span className="tk-convert-path__icon">{folder ? <FolderOpen size={21} strokeWidth={1.7} /> : <FileAudio size={21} strokeWidth={1.7} />}</span>
      <div>
        <span>{label}</span>
        <strong title={path || label}>{path ? displayFileName(path) : label}</strong>
        <small>{path ? path : description}</small>
      </div>
      <ProductButton type="button" tone={path ? "secondary" : "primary"} loading={busy} onClick={onBrowse}>
        <FolderOpen size={14} strokeWidth={1.8} />
        {actionLabel}
      </ProductButton>
      {path ? (
        <button className="tk-subtle-icon-button" type="button" onClick={onClear} title={`Clear ${label.toLowerCase()}`} aria-label={`Clear ${label.toLowerCase()}`}>
          <X size={14} strokeWidth={1.9} />
        </button>
      ) : null}
      {path && !folder ? <LocalAudioPlayer path={path} compact label={`${label} preview`} /> : null}
    </div>
  );
}

type NumberFieldProps = {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  suffix?: string;
  onChange: (value: number) => void;
};

export function NumberField({ label, value, min, max, step, suffix, onChange }: NumberFieldProps) {
  return (
    <label className="tk-field tk-number-field">
      <span className="tk-field__label">{label}</span>
      <input
        className="tk-input"
        type="number"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(event) => onChange(Number(event.target.value))}
      />
      <span className="tk-field__hint">{min}–{max}{suffix ? ` ${suffix}` : ""}</span>
    </label>
  );
}

type ReviewItemProps = {
  checked: boolean;
  label: string;
  onChange: (checked: boolean) => void;
};

export function ReviewItem({ checked, label, onChange }: ReviewItemProps) {
  return (
    <label className={checked ? "tk-review-item is-checked" : "tk-review-item"}>
      <input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} />
      <span>{checked ? <Check size={12} strokeWidth={2.2} /> : null}</span>
      <strong>{label}</strong>
    </label>
  );
}

export function displayFileName(path: string): string {
  const normalized = path.trim().replace(/[\\/]+$/, "");
  const parts = normalized.split(/[\\/]/).filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : "Selected target";
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value.toFixed(value >= 10 ? 1 : 2)} ${unit}`;
}

export const f0Options = [
  { value: "rmvpe", label: "RMVPE" },
  { value: "harvest", label: "Harvest" },
  { value: "crepe", label: "CREPE" },
  { value: "pm", label: "Parselmouth" }
];

export const emptyReview = {
  words: false,
  timbre: false,
  similarity: false,
  artifacts: false
};
