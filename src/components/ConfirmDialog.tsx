import { AlertTriangle } from "lucide-react";
import { useEffect, useRef } from "react";
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
  const confirmButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    confirmButtonRef.current?.focus();
  }, []);

  function handleKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    if (event.key === "Escape") {
      event.preventDefault();
      onCancel();
    }
  }

  return (
    <div className="modal-backdrop">
      <div className="confirm-dialog" role="alertdialog" aria-labelledby="confirm-title" onKeyDown={handleKeyDown}>
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
          <button
            className={destructive ? "button danger" : "button primary"}
            ref={confirmButtonRef}
            onClick={onConfirm}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
