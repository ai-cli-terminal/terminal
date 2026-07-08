import { invoke } from "@tauri-apps/api/core";
import { eofSessionIds, terminalRoot } from "./app_context";
import { getActivePaneSession, primarySession, setStatus, term } from "./pane_session";
import type { PaneSession } from "./types";

let resizeTimer: number | undefined;

export function fitTerminal(): void {
  const session = getActivePaneSession();
  if (!session || session.root.closest<HTMLElement>(".pane")?.hidden) {
    return;
  }
  session.fitAddon.fit();
}

export async function resizeBackend(): Promise<void> {
  const session = getActivePaneSession();
  if (!session?.sessionId) {
    return;
  }

  await invoke("terminal_resize", {
    id: session.sessionId,
    rows: session.terminal.rows,
    cols: session.terminal.cols
  });
}

export function scheduleResize(): void {
  window.clearTimeout(resizeTimer);
  resizeTimer = window.setTimeout(() => {
    fitTerminal();
    void resizeBackend().catch((error: unknown) => {
      setStatus(String(error));
    });
  }, 40);
}

export async function writeToBackend(session: PaneSession, data: string): Promise<void> {
  if (!session.sessionId || !session.isRunning) {
    return;
  }

  await invoke("terminal_write", {
    id: session.sessionId,
    data
  });
}

export async function requestTerminalEof(session: PaneSession): Promise<void> {
  if (!session.sessionId || !session.isRunning) {
    return;
  }

  const id = session.sessionId;
  eofSessionIds.add(id);
  setStatus("exiting");
  await invoke("terminal_eof", { id });
}

export async function handleTerminalInput(session: PaneSession, data: string): Promise<void> {
  if (data === "\x04") {
    await requestTerminalEof(session);
    return;
  }

  await writeToBackend(session, data);
}

export function copySelection(session: PaneSession, event?: ClipboardEvent): string {
  const selection = session.terminal.getSelection();
  if (!selection) {
    return "";
  }

  if (event?.clipboardData) {
    event.preventDefault();
    event.clipboardData.setData("text/plain", selection);
    return selection;
  }

  if (navigator.clipboard) {
    void navigator.clipboard
      .writeText(selection)
      .catch((error: unknown) => setStatus(String(error)));
  }
  return selection;
}

export async function pasteText(session: PaneSession, data: string): Promise<boolean> {
  if (!session.isRunning || data.length === 0) {
    return false;
  }

  await writeToBackend(session, data);
  return true;
}

export function bufferContains(text: string): boolean {
  const buffer = term.buffer.active;
  for (let index = 0; index < buffer.length; index += 1) {
    const line = buffer.getLine(index);
    if (line?.translateToString(true).includes(text)) {
      return true;
    }
  }
  return false;
}

export function findBufferText(text: string): { column: number; row: number } | null {
  const buffer = term.buffer.active;
  for (let row = 0; row < buffer.length; row += 1) {
    const line = buffer.getLine(row)?.translateToString(true);
    if (!line) {
      continue;
    }
    const column = line.indexOf(text);
    if (column >= 0) {
      return { column, row };
    }
  }
  return null;
}

export function dispatchPasteEvent(data: string): boolean {
  if (typeof DataTransfer === "undefined" || typeof ClipboardEvent === "undefined") {
    return false;
  }

  const clipboardData = new DataTransfer();
  clipboardData.setData("text/plain", data);
  const event = new ClipboardEvent("paste", {
    bubbles: true,
    cancelable: true,
    clipboardData
  });
  terminalRoot.dispatchEvent(event);
  return true;
}

export function readCopyEventData(): { copiedText: string; usedEventClipboard: boolean } {
  if (typeof DataTransfer === "undefined" || typeof ClipboardEvent === "undefined") {
    return { copiedText: copySelection(primarySession), usedEventClipboard: false };
  }

  const clipboardData = new DataTransfer();
  const event = new ClipboardEvent("copy", {
    bubbles: true,
    cancelable: true,
    clipboardData
  });
  terminalRoot.dispatchEvent(event);
  return {
    copiedText: clipboardData.getData("text/plain"),
    usedEventClipboard: true
  };
}
