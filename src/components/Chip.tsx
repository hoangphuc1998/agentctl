import type { ReactNode } from "react";

export type ChipTone = "neutral" | "success" | "warning" | "info" | "danger";

interface ChipProps {
  tone?: ChipTone;
  icon?: ReactNode;
  title?: string;
  children: ReactNode;
}

export function Chip({ tone = "neutral", icon, title, children }: ChipProps) {
  return (
    <span className={`chip chip-${tone}`} title={title}>
      {icon && (
        <span className="chip-icon" aria-hidden="true">
          {icon}
        </span>
      )}
      <span className="chip-label">{children}</span>
    </span>
  );
}
