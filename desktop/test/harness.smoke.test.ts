import { describe, expect, it } from "vitest";
import { isRuntimeId } from "../src/workspace_state";
import { aiCliProbeIds } from "../src/runtimes";
import { runtimeLaunchSummary } from "../src/layout";
import { paneLaunchKey } from "../src/pane_session";
import { runtimeLabels } from "../src/app_context";

describe("harness smoke", () => {
  it("loads the source module graph under happy-dom + mocks", () => {
    expect(typeof isRuntimeId).toBe("function");
    expect(typeof aiCliProbeIds).toBe("function");
    expect(typeof runtimeLaunchSummary).toBe("function");
    expect(typeof paneLaunchKey).toBe("function");
    expect(typeof runtimeLabels).toBe("object");
  });
});
