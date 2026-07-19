import { describe, expect, it } from "vitest";
import { runtimeLabels, runtimeNotes } from "../src/app_context";

const RUNTIME_IDS = ["ash", "ubuntu", "docker", "codex", "claude", "gemini"] as const;

describe("runtime label/note maps", () => {
  it("has a non-empty label for every RuntimeId", () => {
    for (const id of RUNTIME_IDS) {
      expect(runtimeLabels[id]).toBeTruthy();
      expect(typeof runtimeLabels[id]).toBe("string");
    }
    expect(Object.keys(runtimeLabels).sort()).toEqual([...RUNTIME_IDS].sort());
  });

  it("has a non-empty note for every RuntimeId", () => {
    for (const id of RUNTIME_IDS) {
      expect(runtimeNotes[id]).toBeTruthy();
      expect(typeof runtimeNotes[id]).toBe("string");
    }
    expect(Object.keys(runtimeNotes).sort()).toEqual([...RUNTIME_IDS].sort());
  });
});
