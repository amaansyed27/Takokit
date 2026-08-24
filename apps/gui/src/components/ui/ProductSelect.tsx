import { ChevronDown } from "lucide-react";
import type { SelectHTMLAttributes } from "react";

type ProductSelectOption = {
  value: string;
  label: string;
};

type ProductSelectProps = SelectHTMLAttributes<HTMLSelectElement> & {
  label: string;
  hint?: string;
  options: ProductSelectOption[];
};

export function ProductSelect({ label, hint, options, className = "", ...props }: ProductSelectProps) {
  return (
    <label className={`tk-field ${className}`.trim()}>
      <span className="tk-field__label">{label}</span>
      <span className="tk-select-wrap">
        <select className="tk-select" {...props}>
          {options.map((option) => (
            <option key={option.value} value={option.value}>{option.label}</option>
          ))}
        </select>
        <ChevronDown size={15} strokeWidth={1.8} aria-hidden="true" />
      </span>
      {hint ? <small className="tk-field__hint">{hint}</small> : null}
    </label>
  );
}
