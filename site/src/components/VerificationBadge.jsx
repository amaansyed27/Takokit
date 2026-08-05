import { STATUS_LABELS } from "../models/presentation";

export function VerificationBadge({ status }) {
  return (
    <span className={`status-badge status-${status}`}>
      {STATUS_LABELS[status] || "Not declared"}
    </span>
  );
}
