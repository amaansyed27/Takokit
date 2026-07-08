import type { ReactElement, ReactNode } from "react";

type TooltipProps = {
  content: string;
  children: ReactElement;
};

export function Tooltip({ content, children }: TooltipProps) {
  return (
    <span className="tooltip" data-tooltip={content}>
      {children}
      <span className="tooltip__bubble" role="tooltip">
        {content}
      </span>
    </span>
  );
}

