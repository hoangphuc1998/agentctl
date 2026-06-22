import type { ObservedState } from "../types";

interface StatusBadgeProps {
  state: ObservedState;
  compact?: boolean;
}

const labels: Record<ObservedState, string> = {
  running: "Running",
  "needs-user": "Needs user",
  "completed-unchecked": "Complete",
  "completed-seen": "Seen",
  unknown: "Unknown"
};

const marks: Record<ObservedState, string> = {
  running: "●",
  "needs-user": "◐",
  "completed-unchecked": "✓",
  "completed-seen": "✓",
  unknown: "?"
};

export function StatusBadge({ state, compact }: StatusBadgeProps) {
  return (
    <span className={`status-badge ${state}`} title={labels[state]}>
      <span aria-hidden="true">{marks[state]}</span>
      {!compact && <span>{labels[state]}</span>}
    </span>
  );
}

