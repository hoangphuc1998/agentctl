import { AlertTriangle } from "lucide-react";
import { Chip } from "./Chip";

interface ConfirmDialogProps {
  title: string;
  body: string;
  confirmLabel: string;
  destructive?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

export function ConfirmDialog({ title, body, confirmLabel, destructive, onCancel, onConfirm }: ConfirmDialogProps) {
  return (
    <div className="modal-backdrop">
      <div className="confirm-dialog" role="alertdialog" aria-labelledby="confirm-title">
        <div className="confirm-title-row">
          <AlertTriangle size={18} />
          <h2 id="confirm-title">{title}</h2>
          <Chip tone={destructive ? "danger" : "info"}>{destructive ? "destructive" : "confirm"}</Chip>
        </div>
        <p>{body}</p>
        <div className="modal-actions">
          <button className="button secondary" onClick={onCancel}>
            Cancel
          </button>
          <button className={destructive ? "button danger" : "button primary"} onClick={onConfirm}>
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
