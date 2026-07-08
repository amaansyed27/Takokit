import type { ReactNode } from "react";

type TableProps = {
  columns: string[];
  children: ReactNode;
  ariaLabel: string;
  compact?: boolean;
};

type TableRowProps = {
  children: ReactNode;
};

export function Table({ columns, children, ariaLabel, compact = false }: TableProps) {
  return (
    <div className={compact ? "table table--compact" : "table"} role="table" aria-label={ariaLabel}>
      <div className="table__row table__head" role="row">
        {columns.map((column) => (
          <span key={column}>{column}</span>
        ))}
      </div>
      {children}
    </div>
  );
}

export function TableRow({ children }: TableRowProps) {
  return (
    <div className="table__row" role="row" tabIndex={0}>
      {children}
    </div>
  );
}

