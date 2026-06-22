import { Activity, AlertTriangle, CheckCircle2, CircleHelp, Eye } from "lucide-react";
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

const icons: Record<ObservedState, typeof Activity> = {
  running: Activity,
  "needs-user": AlertTriangle,
  "completed-unchecked": CheckCircle2,
  "completed-seen": Eye,
  unknown: CircleHelp
};

export function StatusBadge({ state, compact }: StatusBadgeProps) {
  const Icon = icons[state];

  return (
    <span className={`status-badge ${state}`} aria-label={labels[state]} title={labels[state]}>
      <Icon size={14} strokeWidth={2.3} aria-hidden="true" />
      {!compact && <span>{labels[state]}</span>}
    </span>
  );
}
