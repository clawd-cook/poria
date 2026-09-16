import { spawn } from "node:child_process";
import type { IResource, CapabilityMetadata, ResourceContext } from "@poria/core";

// ---------- Types ----------

export interface TerminalExecInput {
  command: string;
  cwd?: string;
  env?: Record<string, string>;
  timeoutMs?: number;
}

export interface TerminalExecResult {
  code: number;
  stdout: string;
  stderr: string;
}

// ---------- TimeoutError ----------

export class TimeoutError extends Error {
  override readonly name = "TimeoutError" as const;

  constructor(
    public readonly command: string,
    public readonly timeoutMs: number,
  ) {
    super(`Command timed out after ${timeoutMs}ms: ${command}`);
  }
}

// ---------- Defaults ----------

const DEFAULT_TIMEOUT_MS = 30_000;

// ---------- Terminal Resource ----------

export class TerminalResource implements IResource<TerminalExecInput, TerminalExecResult> {
  readonly metadata: CapabilityMetadata = {
    id: "resource:terminal",
    name: "Terminal",
    description: "Shell command execution with timeout support",
    version: "0.1.0",
  };

  async execute(input: TerminalExecInput, _context: ResourceContext): Promise<TerminalExecResult> {
    return exec(input);
  }
}

/**
 * Execute a shell command with timeout support.
 * Migrated from submodules/poria/packages/resource-terminal — spawn + timeout logic.
 */
export function exec(input: TerminalExecInput): Promise<TerminalExecResult> {
  const timeoutMs =
    input.timeoutMs != null && input.timeoutMs > 0
      ? input.timeoutMs
      : DEFAULT_TIMEOUT_MS;

  return new Promise((resolve, reject) => {
    const child = spawn(input.command, {
      cwd: input.cwd,
      env: { ...process.env, ...input.env },
      shell: true,
      windowsHide: true,
    });

    let stdout = "";
    let stderr = "";
    let settled = false;

    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      child.kill("SIGKILL");
      reject(new TimeoutError(input.command, timeoutMs));
    }, timeoutMs);

    child.stdout?.on("data", (chunk: Buffer) => {
      stdout += chunk.toString();
    });

    child.stderr?.on("data", (chunk: Buffer) => {
      stderr += chunk.toString();
    });

    child.on("error", (error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      reject(error);
    });

    child.on("close", (code) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve({
        code: code ?? 1,
        stdout,
        stderr,
      });
    });
  });
}
