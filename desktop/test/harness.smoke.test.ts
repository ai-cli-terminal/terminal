import { beforeAll, describe, expect, it } from "vitest";
import type { PaneModel } from "../src/types";

// The source module graph is circular:
//   layout → workspace_state → runtimes → layout
//   layout → pane_session → app_context
// layout.ts:43 runs loadWorkspaceState() at module-load time, inside the cycle.
// Under Vitest's SSR transform, whichever cyclic module is imported *first*
// decides whether the bindings needed at layout:43 are already initialized.
// In the real app (and native ESM) the entry point (main → app_context → ...)
// primes a stable order. We reproduce that here by warming the module graph in
// dependency order in beforeAll, so every test then imports the REAL, fully
// initialized implementations — no source-module stubs.
//
// Assertions check ACTUAL behavior of the real pure functions, not just
// `typeof`. If someone later replaces these modules with function stubs, these
// assertions go red.

beforeAll(async () => {
  // Warm leaves/producers before consumers so the circular bindings are all
  // populated before layout.ts:43 (loadWorkspaceState) executes.
  await import("../src/app_context");
  await import("../src/runtimes");
  await import("../src/pane_session");
  await import("../src/workspace_state");
  await import("../src/layout");
});

describe("harness smoke", () => {
  it("loads real workspace_state.isRuntimeId with correct predicate behavior", async () => {
    const { isRuntimeId } = await import("../src/workspace_state");
    expect(isRuntimeId("ash")).toBe(true);
    expect(isRuntimeId("ubuntu")).toBe(true);
    expect(isRuntimeId("docker")).toBe(true);
    expect(isRuntimeId("codex")).toBe(true);
    expect(isRuntimeId("claude")).toBe(true);
    expect(isRuntimeId("gemini")).toBe(true);
    expect(isRuntimeId("nope")).toBe(false);
    expect(isRuntimeId("")).toBe(false);
    expect(isRuntimeId(null)).toBe(false);
    expect(isRuntimeId(42)).toBe(false);
  });

  it("loads real runtimes.aiCliProbeIds returning the AI CLI ids", async () => {
    const { aiCliProbeIds } = await import("../src/runtimes");
    expect(aiCliProbeIds()).toEqual(["codex", "claude", "gemini"]);
  });

  it("loads real pane_session.paneLaunchKey computing the launch key", async () => {
    const { paneLaunchKey } = await import("../src/pane_session");
    const codexPane: PaneModel = {
      id: "pane-x",
      title: "Pane X",
      runtime: "codex",
      dockerAppId: "ubuntu-base",
      aptPackageId: "git",
      workspaceDir: "/w"
    };
    expect(paneLaunchKey(codexPane)).toBe("codex|/w");

    const ashPane: PaneModel = { ...codexPane, runtime: "ash", workspaceDir: "" };
    expect(paneLaunchKey(ashPane)).toBe("ash");
  });

  it("loads real layout.runtimeLaunchSummary returning a real summary string", async () => {
    const { runtimeLaunchSummary } = await import("../src/layout");
    const summary = runtimeLaunchSummary("ash", null);
    expect(typeof summary).toBe("string");
    expect(summary).toContain("starting ash");
  });

  it("loads real app_context.runtimeLabels with actual label mapping", async () => {
    const { runtimeLabels } = await import("../src/app_context");
    expect(runtimeLabels.ash).toBe("ash");
    expect(runtimeLabels.ubuntu).toBe("Ubuntu");
    expect(runtimeLabels.docker).toBe("Docker");
    expect(runtimeLabels.codex).toBe("Codex");
    expect(runtimeLabels.claude).toBe("Claude");
    expect(runtimeLabels.gemini).toBe("Gemini");
  });
});
