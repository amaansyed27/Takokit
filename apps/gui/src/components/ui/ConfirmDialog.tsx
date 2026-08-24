import type { ReactNode } from "react";
import { AlertTriangle, X } from "lucide-react";
import { ProductButton } from "./ProductButton";

type ConfirmDialogProps = {
  open: boolean;
  title: string;
  description: ReactNode;
  confirmLabel: string;
  busy?: boolean;
  destructive?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
};

export function ConfirmDialog({
  open,
  title,
  description,
  confirmLabel,
  busy = false,
  destructive = false,
  onCancel,
  onConfirm
}: ConfirmDialogProps) {
  if (!open) return null;

  return (
    <div className="tk-dialog-backdrop" role="presentation" onMouseDown={onCancel}>
      <div
        className="tk-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="tk-confirm-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="tk-dialog__header">
          <span className={destructive ? "tk-dialog__icon is-danger" : "tk-dialog__icon"}>
            <AlertTriangle size={18} strokeWidth={1.9} />
          </span>
          <div>
            <strong id="tk-confirm-title">{title}</strong>
            <div className="tk-dialog__description">{description}</div>
          </div>
          <button className="tk-dialog__close" type="button" onClick={onCancel} aria-label="Close confirmation">
            <X size={16} strokeWidth={1.8} />
          </button>
        </div>
        <div className="tk-dialog__actions">
          <ProductButton tone="secondary" type="button" disabled={busy} onClick={onCancel}>
            Cancel
          </ProductButton>
          <ProductButton tone={destructive ? "danger" : "primary"} type="button" loading={busy} onClick={onConfirm}>
            {confirmLabel}
          </ProductButton>
        </div>
      </div>
    </div>
  );
}
