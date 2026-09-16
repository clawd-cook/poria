import { describe, it, expect, vi } from "vitest";
import { CredentialGuard, AuthExpiredDuringPipelineError } from "../credential-guard.js";
import type { JacpCredentials } from "../credentials.js";
import type { ApiProbe } from "../credential-guard.js";

describe("CredentialGuard", () => {
  const validCreds: JacpCredentials = {
    username: "testuser",
    cookie: "erp_erp=testuser; session=abc123",
  };

  it("should return credentials when probe succeeds", async () => {
    const probe: ApiProbe = vi.fn().mockResolvedValue(undefined);
    const guard = new CredentialGuard(probe);

    const result = await guard.ensureValid(validCreds);
    expect(result).toBe(validCreds);
    expect(probe).toHaveBeenCalledTimes(1);
  });

  it("should throw non-auth errors through", async () => {
    const probe: ApiProbe = vi.fn().mockRejectedValue(new Error("Network timeout"));
    const guard = new CredentialGuard(probe);

    await expect(guard.ensureValid(validCreds)).rejects.toThrow("Network timeout");
  });

  it("should throw AuthExpiredDuringPipelineError when auth fails and refresh fails", async () => {
    // Probe returns 401 auth error
    const probe: ApiProbe = vi.fn().mockRejectedValue(new Error("401 Unauthorized"));
    const guard = new CredentialGuard(probe);

    // readBrowserCookies will fail (no @rookie-rs/api installed in test)
    await expect(guard.ensureValid(validCreds)).rejects.toBeInstanceOf(
      AuthExpiredDuringPipelineError,
    );
  });

  it("AuthExpiredDuringPipelineError should have correct name and message", () => {
    const error = new AuthExpiredDuringPipelineError();
    expect(error.name).toBe("AuthExpiredDuringPipelineError");
    expect(error.message).toContain("poria auth login");
  });

  it("AuthExpiredDuringPipelineError should accept custom message", () => {
    const error = new AuthExpiredDuringPipelineError("Custom message");
    expect(error.message).toBe("Custom message");
  });
});
