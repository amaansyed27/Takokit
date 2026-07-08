import type { ButtonHTMLAttributes, ReactNode } from "react";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  children: ReactNode;
  variant?: "primary" | "secondary" | "ghost";
  loading?: boolean;
};

export function Button({ children, variant = "secondary", loading = false, disabled, className = "", ...props }: ButtonProps) {
  return (
    <button className={`button button--${variant} ${className}`} disabled={disabled || loading} {...props}>
      {loading && <span className="button__spinner" aria-hidden="true" />}
      <span>{children}</span>
    </button>
  );
}

