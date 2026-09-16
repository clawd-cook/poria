import { describe, it, expect } from "vitest";
import { exec, TimeoutError, TerminalResource } from "../index.js";

describe("TerminalResource", () => {
  describe("exec()", () => {
    it("returns stdout, stderr, and exit code for a normal command", async () => {
      const result = await exec({
        command: 'echo "hello world"',
        timeoutMs: 5000,
      });

      expect(result.code).toBe(0);
      expect(result.stdout.trim()).toBe("hello world");
      expect(result.stderr).toBe("");
    });

    it("captures stderr output", async () => {
      const result = await exec({
        command: 'echo "err" >&2',
        timeoutMs: 5000,
      });

      expect(result.code).toBe(0);
      expect(result.stderr.trim()).toBe("err");
    });

    it("returns non-zero exit code for failing commands", async () => {
      const result = await exec({
        command: "exit 42",
        timeoutMs: 5000,
      });

      expect(result.code).toBe(42);
    });

    it("throws TimeoutError when command exceeds timeout", async () => {
      const start = Date.now();

      await expect(
        exec({
          command: "sleep 60",
          timeoutMs: 200,
        }),
      ).rejects.toThrow(TimeoutError);

      const elapsed = Date.now() - start;
      // Should have timed out well under 5 seconds
      expect(elapsed).toBeLessThan(5000);
    });

    it("TimeoutError contains command and timeout info", async () => {
      try {
        await exec({ command: "sleep 60", timeoutMs: 100 });
        // Should not reach here
        expect.fail("Expected TimeoutError");
      } catch (err) {
        expect(err).toBeInstanceOf(TimeoutError);
        const te = err as TimeoutError;
        expect(te.command).toBe("sleep 60");
        expect(te.timeoutMs).toBe(100);
        expect(te.name).toBe("TimeoutError");
        expect(te.message).toContain("100ms");
      }
    });

    it("respects cwd option", async () => {
      const result = await exec({
        command: "pwd",
        cwd: "/tmp",
        timeoutMs: 5000,
      });

      // macOS may resolve /tmp to /private/tmp
      expect(result.stdout.trim()).toMatch(/\/tmp$/);
    });

    it("uses default timeout when timeoutMs is not provided", async () => {
      // Just verify it does not throw for a fast command
      const result = await exec({ command: 'echo "ok"' });
      expect(result.code).toBe(0);
      expect(result.stdout.trim()).toBe("ok");
    });
  });

  describe("TerminalResource class", () => {
    it("has correct metadata", () => {
      const resource = new TerminalResource();
      expect(resource.metadata.id).toBe("resource:terminal");
      expect(resource.metadata.name).toBe("Terminal");
      expect(resource.metadata.version).toBe("0.1.0");
    });

    it("execute() delegates to exec()", async () => {
      const resource = new TerminalResource();
      const result = await resource.execute(
        { command: 'echo "via resource"', timeoutMs: 5000 },
        {},
      );
      expect(result.code).toBe(0);
      expect(result.stdout.trim()).toBe("via resource");
    });
  });
});
