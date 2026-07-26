import type { ILink, ILinkProvider } from "@xterm/xterm";

export type TerminalLinkTarget =
  | { kind: "url"; url: string }
  | { kind: "file"; path: string; line?: number; column?: number };

export type TerminalLinkActivation = "direct" | "modifier";

export interface DetectedTerminalLink {
  text: string;
  startIndex: number;
  endIndex: number;
  target: TerminalLinkTarget;
  activation: TerminalLinkActivation;
}

interface TerminalBufferReader {
  buffer: {
    active: {
      getLine(index: number):
        | { translateToString(trimRight?: boolean): string }
        | undefined;
    };
  };
}

interface TerminalLinkInteraction {
  activate(
    target: TerminalLinkTarget,
    event: MouseEvent,
    activation: TerminalLinkActivation
  ): void;
  hover?(target: TerminalLinkTarget, activation: TerminalLinkActivation): void;
  leave?(target: TerminalLinkTarget): void;
}

const TERMINAL_TOKEN = /\S+/g;
const FILE_LOCATION = /(?::(\d+)(?::(\d+))?|#L(\d+)(?::?C?(\d+))?)$/i;
const FILE_NAME = /(?:^|\/)(?:[^/]+\.[a-z][a-z0-9_-]{0,15}|Dockerfile|Makefile)$/i;

export function detectTerminalLinks(line: string): DetectedTerminalLink[] {
  const links: DetectedTerminalLink[] = [];

  for (const match of line.matchAll(TERMINAL_TOKEN)) {
    const raw = match[0];
    const rawStart = match.index ?? 0;
    const token = trimTerminalToken(raw);
    if (!token.text) continue;

    const detected = targetFromToken(token.text);
    if (!detected) continue;

    const startIndex = rawStart + token.startOffset;
    links.push({
      text: token.text,
      startIndex,
      endIndex: startIndex + token.text.length,
      ...detected
    });
  }

  return links;
}

export function terminalLinkTargetFromUri(uri: string): TerminalLinkTarget | null {
  let parsed: URL;
  try {
    parsed = new URL(uri);
  } catch {
    return null;
  }

  if (parsed.protocol === "http:" || parsed.protocol === "https:") {
    return { kind: "url", url: parsed.toString() };
  }
  if (parsed.protocol !== "file:" || (parsed.hostname && parsed.hostname !== "localhost")) {
    return null;
  }

  const decodedPath = decodeURIComponent(parsed.pathname);
  const pathLocation = fileTargetFromPath(decodedPath);
  const hashLocation = fileLocationFromHash(parsed.hash);
  return {
    kind: "file",
    path: pathLocation.path,
    ...pathLocation.location,
    ...hashLocation
  };
}

export function terminalLinkActivationFromUri(uri: string): TerminalLinkActivation {
  return terminalLinkTargetFromUri(uri)?.kind === "file" ? "direct" : "modifier";
}

export function createTerminalLinkProvider(
  terminal: TerminalBufferReader,
  interaction: TerminalLinkInteraction
): ILinkProvider {
  return {
    provideLinks(bufferLineNumber, callback) {
      const line = terminal.buffer.active.getLine(bufferLineNumber - 1);
      if (!line) {
        callback(undefined);
        return;
      }

      const links: ILink[] = detectTerminalLinks(line.translateToString(true)).map((link) => ({
        text: link.text,
        range: {
          start: { x: link.startIndex + 1, y: bufferLineNumber },
          end: { x: link.endIndex, y: bufferLineNumber }
        },
        activate: (event) => interaction.activate(link.target, event, link.activation),
        hover: () => interaction.hover?.(link.target, link.activation),
        leave: () => interaction.leave?.(link.target)
      }));
      callback(links.length ? links : undefined);
    }
  };
}

export function shouldOpenTerminalLink(
  event: Pick<MouseEvent, "button" | "ctrlKey">,
  activation: TerminalLinkActivation = "modifier"
): boolean {
  return event.button === 0 && (activation === "direct" || event.ctrlKey);
}

function targetFromToken(token: string): {
  target: TerminalLinkTarget;
  activation: TerminalLinkActivation;
} | null {
  if (/^https?:\/\//i.test(token)) {
    const target = terminalLinkTargetFromUri(token);
    return target ? { target, activation: "modifier" } : null;
  }
  if (/^file:\/\//i.test(token)) {
    const target = terminalLinkTargetFromUri(token);
    return target ? { target, activation: "direct" } : null;
  }

  const { path, location } = fileTargetFromPath(token);
  if (!isFilePath(path)) return null;
  return {
    target: {
      kind: "file",
      path,
      ...location
    },
    activation: "modifier"
  };
}

function fileTargetFromPath(pathWithLocation: string): {
  path: string;
  location: { line?: number; column?: number };
} {
  const match = pathWithLocation.match(FILE_LOCATION);
  const path = match ? pathWithLocation.slice(0, -match[0].length) : pathWithLocation;
  const line = numberFromLocation(match?.[1] ?? match?.[3]);
  const column = numberFromLocation(match?.[2] ?? match?.[4]);
  return {
    path,
    location: {
      ...(line ? { line } : {}),
      ...(column ? { column } : {})
    }
  };
}

function isFilePath(path: string): boolean {
  if (!path || path.includes(":")) return false;
  if (!/^[\p{L}\p{N}._~@+()\-/]+$/u.test(path)) return false;
  return (
    path.startsWith("/") ||
    path.startsWith("./") ||
    path.startsWith("../") ||
    path.includes("/") ||
    FILE_NAME.test(path)
  );
}

function fileLocationFromHash(hash: string): { line?: number; column?: number } {
  const match = hash.match(/^#L(\d+)(?::?C?(\d+))?$/i);
  const line = numberFromLocation(match?.[1]);
  const column = numberFromLocation(match?.[2]);
  return {
    ...(line ? { line } : {}),
    ...(column ? { column } : {})
  };
}

function numberFromLocation(value: string | undefined): number | undefined {
  if (!value) return undefined;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : undefined;
}

function trimTerminalToken(raw: string): { text: string; startOffset: number } {
  let startOffset = 0;
  let endOffset = raw.length;
  while (startOffset < endOffset && /[([{<'"]/.test(raw[startOffset])) startOffset += 1;
  while (endOffset > startOffset && /[.,;!?'">]/.test(raw[endOffset - 1])) endOffset -= 1;

  for (const [opening, closing] of [
    ["(", ")"],
    ["[", "]"],
    ["{", "}"]
  ]) {
    while (
      raw[endOffset - 1] === closing &&
      countCharacter(raw.slice(startOffset, endOffset), closing) >
        countCharacter(raw.slice(startOffset, endOffset), opening)
    ) {
      endOffset -= 1;
    }
  }

  return { text: raw.slice(startOffset, endOffset), startOffset };
}

function countCharacter(value: string, character: string): number {
  return [...value].filter((candidate) => candidate === character).length;
}
