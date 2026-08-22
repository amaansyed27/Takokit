import { ArrowUpRight } from "lucide-react";
import type { LucideIcon } from "lucide-react";

type ProductActionCardProps = {
  icon: LucideIcon;
  title: string;
  description: string;
  meta?: string;
  onClick: () => void;
};

export function ProductActionCard({ icon: Icon, title, description, meta, onClick }: ProductActionCardProps) {
  return (
    <button className="tk-action-card" type="button" onClick={onClick}>
      <span className="tk-action-card__icon"><Icon size={20} strokeWidth={1.7} aria-hidden="true" /></span>
      <span className="tk-action-card__body">
        <strong>{title}</strong>
        <span>{description}</span>
        {meta ? <small>{meta}</small> : null}
      </span>
      <ArrowUpRight className="tk-action-card__arrow" size={18} strokeWidth={1.6} aria-hidden="true" />
    </button>
  );
}
