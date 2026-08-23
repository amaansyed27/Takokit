import type { ButtonHTMLAttributes, ReactNode } from "react";

type ProductButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  children: ReactNode;
  tone?: "primary" | "secondary" | "ghost" | "danger";
  loading?: boolean;
};

export function ProductButton({
  children,
  tone = "secondary",
  loading = false,
  disabled,
  className = "",
  ...props
}: ProductButtonProps) {
  return (
    <button
      className={`tk-button tk-button--${tone} ${className}`.trim()}
      disabled={disabled || loading}
      {...props}
    >
      {loading ? <span className="tk-button__spinner" aria-hidden="true" /> : null}
      <span className="tk-button__content">{children}</span>
    </button>
  );
}
