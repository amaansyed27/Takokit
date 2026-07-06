import type { TextareaHTMLAttributes } from "react";

type TextAreaProps = TextareaHTMLAttributes<HTMLTextAreaElement> & {
  label: string;
  error?: string;
  count?: string;
};

export function TextArea({ label, error, count, ...props }: TextAreaProps) {
  return (
    <label className={error ? "field field--error" : "field"}>
      <span>{label}</span>
      <textarea {...props} />
      <span className="field__meta">
        {error ? <small>{error}</small> : <small />}
        {count && <small>{count}</small>}
      </span>
    </label>
  );
}

