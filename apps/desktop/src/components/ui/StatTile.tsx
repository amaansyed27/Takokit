import type { ReactNode } from "react";

type StatTileProps = {
  label: string;
  value: ReactNode;
  detail?: string;
};

export function StatTile({ label, value, detail }: StatTileProps) {
  return (
    <div className="stat-tile">
      <span>{label}</span>
      <strong className="stat-tile__value">{value}</strong>
      {detail && <small>{detail}</small>}
    </div>
  );
}

