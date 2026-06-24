export type AppShortcutAction =
  | "open-palette"
  | "new-run"
  | "previous-run"
  | "next-run"
  | "end-run";

export function appShortcutFromEvent(event: KeyboardEvent): AppShortcutAction | null {
  if (isEditableShortcutTarget(event.target)) return null;

  const key = event.key.toLowerCase();
  const commandModifier = event.ctrlKey || event.metaKey;

  if (commandModifier && !event.altKey && !event.shiftKey && key === "k") {
    return "open-palette";
  }

  if (commandModifier && event.shiftKey && !event.altKey && key === "n") {
    return "new-run";
  }

  if (commandModifier && event.shiftKey && !event.altKey && key === "e") {
    return "end-run";
  }

  if (event.altKey && !event.ctrlKey && !event.metaKey && !event.shiftKey) {
    if (event.key === "ArrowUp") return "previous-run";
    if (event.key === "ArrowDown") return "next-run";
  }

  return null;
}

function isEditableShortcutTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.closest(".xterm")) return false;

  const tagName = target.tagName.toLowerCase();
  if (tagName === "input" || tagName === "textarea" || tagName === "select") {
    return true;
  }

  return target.isContentEditable || Boolean(target.closest("[contenteditable='true']"));
}
