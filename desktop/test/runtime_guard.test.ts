import { describe, expect, it } from "vitest";
import { isRuntimeId } from "../src/workspace_state";

describe("isRuntimeId", () => {
  it("accepts every develop RuntimeId", () => {
    for (const id of ["ash", "ubuntu", "docker", "codex", "claude", "gemini"]) {
      expect(isRuntimeId(id)).toBe(true);
    }
  });

  it("rejects unknown or non-string values", () => {
    expect(isRuntimeId("powershell")).toBe(false); // develop 미포함(Draft #109 전용)
    expect(isRuntimeId("bash")).toBe(false);
    expect(isRuntimeId("")).toBe(false);
    expect(isRuntimeId(null)).toBe(false);
    expect(isRuntimeId(undefined)).toBe(false);
    expect(isRuntimeId(42)).toBe(false);
    expect(isRuntimeId({ runtime: "ash" })).toBe(false);
  });
});
