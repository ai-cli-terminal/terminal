import { describe, expect, it } from "vitest";
import { paneLaunchKey } from "../src/pane_session";
import type { PaneModel, RuntimeId } from "../src/types";

function pane(over: Partial<PaneModel> & { runtime: RuntimeId }): PaneModel {
  return {
    id: "pane-1",
    title: "Pane 1",
    dockerAppId: "ubuntu-base",
    aptPackageId: "git",
    workspaceDir: "",
    ...over
  };
}

describe("paneLaunchKey", () => {
  it("docker key includes dockerAppId and workspace dir", () => {
    expect(paneLaunchKey(pane({ runtime: "docker", dockerAppId: "myapp", workspaceDir: "/w" })))
      .toBe("docker|myapp|/w");
  });

  it("ubuntu key includes aptPackageId and workspace dir", () => {
    expect(paneLaunchKey(pane({ runtime: "ubuntu", aptPackageId: "curl", workspaceDir: "/w" })))
      .toBe("ubuntu|curl|/w");
  });

  it("powershell key includes the Windows host workspace dir", () => {
    expect(paneLaunchKey(pane({ runtime: "powershell", workspaceDir: "C:\\work" })))
      .toBe("powershell|C:\\work");
  });

  it("AI CLI keys include runtime and workspace dir only", () => {
    expect(paneLaunchKey(pane({ runtime: "codex", workspaceDir: "/w" }))).toBe("codex|/w");
    expect(paneLaunchKey(pane({ runtime: "claude", workspaceDir: "/w" }))).toBe("claude|/w");
    expect(paneLaunchKey(pane({ runtime: "gemini", workspaceDir: "/w" }))).toBe("gemini|/w");
  });

  it("blank workspace dir collapses to empty segment", () => {
    expect(paneLaunchKey(pane({ runtime: "docker", dockerAppId: "myapp", workspaceDir: "   " })))
      .toBe("docker|myapp|");
    expect(paneLaunchKey(pane({ runtime: "powershell", workspaceDir: "   " })))
      .toBe("powershell|");
  });

  it("ash (fallthrough) key is just the runtime", () => {
    expect(paneLaunchKey(pane({ runtime: "ash", workspaceDir: "/w" }))).toBe("ash");
  });
});
