import { describe, expect, it } from "vitest";
import { runtimeLaunchSummary } from "../src/layout";
import type { PaneModel, RuntimeId } from "../src/types";

function pane(runtime: RuntimeId): PaneModel {
  return { id: "pane-1", title: "Pane 1", runtime, dockerAppId: "ubuntu-base", aptPackageId: "git", workspaceDir: "" };
}

describe("runtimeLaunchSummary", () => {
  it("ash summary", () => {
    expect(runtimeLaunchSummary("ash", pane("ash")))
      .toBe("starting ash (workspace: runtime default)");
  });

  it("ubuntu summary without a resolvable package", () => {
    expect(runtimeLaunchSummary("ubuntu", pane("ubuntu")))
      .toBe("starting Ubuntu (workspace: runtime default)");
  });

  it("docker summary falls back to 'selected Docker app' and docker workspace phrasing", () => {
    expect(runtimeLaunchSummary("docker", pane("docker")))
      .toBe("starting Docker app: selected Docker app (workspace: app working directory -> /workspace)");
  });

  it("AI CLI runtimes route through the managed-Ubuntu branch with their label", () => {
    expect(runtimeLaunchSummary("codex", pane("codex")))
      .toBe("starting Codex CLI in managed Ubuntu (workspace: runtime default)");
    expect(runtimeLaunchSummary("claude", pane("claude")))
      .toBe("starting Claude CLI in managed Ubuntu (workspace: runtime default)");
    expect(runtimeLaunchSummary("gemini", pane("gemini")))
      .toBe("starting Gemini CLI in managed Ubuntu (workspace: runtime default)");
  });

  it("null pane still yields a default-workspace summary", () => {
    expect(runtimeLaunchSummary("codex", null))
      .toBe("starting Codex CLI in managed Ubuntu (workspace: runtime default)");
  });
});
