import { describe, expect, it } from "vitest";
import { aiCliProbeIds, getAiCliProbes, missingAiCliLabels } from "../src/runtimes";
import type { RuntimeInventory, RuntimeProbe } from "../src/types";

function probe(over: Partial<RuntimeProbe> & Pick<RuntimeProbe, "id" | "label" | "status">): RuntimeProbe {
  return { detail: "", ...over };
}

function inventory(probes: RuntimeProbe[]): RuntimeInventory {
  return { checkedAtEpochSeconds: 0, probes };
}

describe("AI CLI probe helpers", () => {
  it("aiCliProbeIds is codex/claude/gemini", () => {
    expect(aiCliProbeIds()).toEqual(["codex", "claude", "gemini"]);
  });

  it("getAiCliProbes returns only present AI CLI probes, ignoring non-AI ones", () => {
    const inv = inventory([
      probe({ id: "ubuntu", label: "Ubuntu", status: "ready" }),
      probe({ id: "codex", label: "Codex", status: "ready" }),
      probe({ id: "claude", label: "Claude", status: "missing" })
      // gemini 없음
    ]);
    expect(getAiCliProbes(inv).map((p) => p.id)).toEqual(["codex", "claude"]);
  });

  it("missingAiCliLabels lists labels of non-ready AI CLI probes", () => {
    const inv = inventory([
      probe({ id: "codex", label: "Codex", status: "ready" }),
      probe({ id: "claude", label: "Claude", status: "missing" }),
      probe({ id: "gemini", label: "Gemini", status: "unavailable" })
    ]);
    expect(missingAiCliLabels(inv)).toEqual(["Claude", "Gemini"]);
  });

  it("missingAiCliLabels is empty when all AI CLIs are ready", () => {
    const inv = inventory([
      probe({ id: "codex", label: "Codex", status: "ready" }),
      probe({ id: "claude", label: "Claude", status: "ready" }),
      probe({ id: "gemini", label: "Gemini", status: "ready" })
    ]);
    expect(missingAiCliLabels(inv)).toEqual([]);
  });
});
