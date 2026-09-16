import { describe, it, expect } from "vitest";
import { isJoySpaceFixtureMode } from "../export.js";

describe("JoySpace channel", () => {
  it("isJoySpaceFixtureMode returns false by default", () => {
    expect(isJoySpaceFixtureMode()).toBe(false);
  });

  it("isJoySpaceFixtureMode returns true when PORIA_JOYSPACE_FIXTURE=1", () => {
    const original = process.env.PORIA_JOYSPACE_FIXTURE;
    try {
      process.env.PORIA_JOYSPACE_FIXTURE = "1";
      expect(isJoySpaceFixtureMode()).toBe(true);
    } finally {
      if (original === undefined) {
        delete process.env.PORIA_JOYSPACE_FIXTURE;
      } else {
        process.env.PORIA_JOYSPACE_FIXTURE = original;
      }
    }
  });
});
