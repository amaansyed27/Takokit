import type { ReactNode } from "react";

type ProductMetricProps = {
  label: string;
  value: ReactNode;
  detail?: string;
};

export function ProductMetric({ label, value, detail }: ProductMetricProps) {
  return (
    <div className="tk-metric">
      <span>{label}</span>
      <strong>{value}</strong>
      {detail ? <small>{detail}</small> : null}
    </div>
  );
}
