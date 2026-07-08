import { invoke } from "@tauri-apps/api/core";
import { primarySession, setStatus, term } from "./pane_session";
import { bufferContains, dispatchPasteEvent, findBufferText, handleTerminalInput, pasteText, readCopyEventData, writeToBackend } from "./terminal_io";
import type { FrontendSmokeConfig, FrontendSmokeEvidence } from "./types";

export async function writeFrontendSmokeEvidence(evidence: FrontendSmokeEvidence): Promise<void> {
  await invoke("terminal_write_smoke_frontend_evidence", {
    evidence: JSON.stringify(evidence, null, 2)
  });
}

export async function runFrontendSmoke(config: FrontendSmokeConfig): Promise<void> {
  const evidence: FrontendSmokeEvidence = {
    status: "failed",
    timestamp: new Date().toISOString(),
    selection: {
      text: config.selectionText,
      selected: false,
      selectedTextLength: 0
    },
    copy: {
      copied: false,
      copiedTextLength: 0,
      usedEventClipboard: false
    },
    paste: {
      text: config.pasteText,
      expectedOutput: config.pasteExpectedOutput,
      dispatched: false
    },
    scrollback: {
      configuredScrollback: term.options.scrollback,
      requestedLines: config.scrollbackLines,
      bufferLength: term.buffer.active.length,
      beforeBaseY: term.buffer.active.baseY,
      afterBaseY: term.buffer.active.baseY,
      viewportAfterTop: term.buffer.active.viewportY,
      viewportAfterBottom: term.buffer.active.viewportY,
      firstMarkerRetained: false,
      lastMarkerRetained: false,
      scrolled: false
    }
  };

  try {
    term.writeln(config.selectionText);
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
    const selectionPosition = findBufferText(config.selectionText);
    if (selectionPosition) {
      term.select(
        selectionPosition.column,
        selectionPosition.row,
        config.selectionText.length
      );
    } else {
      term.selectAll();
    }
    const selectedText = term.getSelection();
    evidence.selection.selectedTextLength = selectedText.length;
    evidence.selection.selected =
      selectionPosition !== null &&
      selectedText.length === config.selectionText.length;

    const copyResult = readCopyEventData();
    evidence.copy.copiedTextLength = copyResult.copiedText.length;
    evidence.copy.usedEventClipboard = copyResult.usedEventClipboard;
    evidence.copy.copied =
      copyResult.copiedText.length > 0 &&
      copyResult.copiedText === selectedText;
    term.clearSelection();

    const pasteDispatched = dispatchPasteEvent(config.pasteText);
    evidence.paste.dispatched =
      pasteDispatched || await pasteText(primarySession, config.pasteText);

    const firstMarker = "AI_TERMINAL_GUI_SMOKE_SCROLLBACK_000";
    const lastMarker =
      `AI_TERMINAL_GUI_SMOKE_SCROLLBACK_${String(config.scrollbackLines - 1).padStart(3, "0")}`;
    evidence.scrollback.beforeBaseY = term.buffer.active.baseY;
    for (let index = 0; index < config.scrollbackLines; index += 1) {
      term.writeln(`AI_TERMINAL_GUI_SMOKE_SCROLLBACK_${String(index).padStart(3, "0")}`);
    }
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));

    evidence.scrollback.afterBaseY = term.buffer.active.baseY;
    evidence.scrollback.bufferLength = term.buffer.active.length;
    evidence.scrollback.firstMarkerRetained = bufferContains(firstMarker);
    evidence.scrollback.lastMarkerRetained = bufferContains(lastMarker);
    term.scrollToTop();
    evidence.scrollback.viewportAfterTop = term.buffer.active.viewportY;
    term.scrollToBottom();
    evidence.scrollback.viewportAfterBottom = term.buffer.active.viewportY;
    evidence.scrollback.scrolled =
      evidence.scrollback.afterBaseY > evidence.scrollback.beforeBaseY &&
      evidence.scrollback.viewportAfterTop !== evidence.scrollback.viewportAfterBottom;

    evidence.status =
      evidence.selection.selected &&
      evidence.copy.copied &&
      evidence.paste.dispatched &&
      evidence.scrollback.firstMarkerRetained &&
      evidence.scrollback.lastMarkerRetained &&
      evidence.scrollback.scrolled
        ? "passed"
        : "failed";
  } finally {
    await writeFrontendSmokeEvidence(evidence);
  }
}

export async function scheduleFrontendSmokeIfConfigured(): Promise<void> {
  const config = await invoke<FrontendSmokeConfig | null>("terminal_smoke_frontend_config");
  if (!config) {
    return;
  }

  window.setTimeout(() => {
    void runFrontendSmoke(config).catch((error: unknown) => {
      setStatus(String(error));
    });
  }, config.delayMilliseconds);
}

export async function writeSmokeCommandIfConfigured(): Promise<void> {
  const [command, ctrlDDelayMs] = await Promise.all([
    invoke<string | null>("terminal_smoke_command"),
    invoke<number | null>("terminal_smoke_ctrl_d_delay_ms")
  ]);

  if (!command && ctrlDDelayMs === null) {
    return;
  }

  if (command) {
    const data = command.endsWith("\r") || command.endsWith("\n")
      ? command
      : `${command}\r`;
    window.setTimeout(() => {
      void writeToBackend(primarySession, data).catch((error: unknown) => {
        setStatus(String(error));
      });
    }, 250);
  }

  if (ctrlDDelayMs !== null) {
    window.setTimeout(() => {
      void handleTerminalInput(primarySession, "\x04").catch((error: unknown) => {
        setStatus(String(error));
      });
    }, ctrlDDelayMs);
  }
}
