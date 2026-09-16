import { spawn, type ChildProcess } from "node:child_process";
import { homedir } from "node:os";
import path from "node:path";

const DEFAULT_NODE_PATH = path.join(
  homedir(),
  ".joyclaw/node/node-v22.16.0-darwin-arm64/bin/node",
);
const DEFAULT_OPENCLAW_PATH = path.join(
  homedir(),
  ".joyclaw/apps/v2.5.6/openclaw.mjs",
);
const GATEWAY_PORT = 18810;

export interface JoyClawBridgeConfig {
  nodePath?: string;
  openclawPath?: string;
  gatewayPort?: number;
}

function getNodePath(config?: JoyClawBridgeConfig): string {
  return config?.nodePath || process.env.PORIA_JOYCLAW_NODE_PATH || DEFAULT_NODE_PATH;
}

function getOpenclawPath(config?: JoyClawBridgeConfig): string {
  return config?.openclawPath || process.env.PORIA_JOYCLAW_OPENCLAW_PATH || DEFAULT_OPENCLAW_PATH;
}

function getGatewayPort(config?: JoyClawBridgeConfig): number {
  return config?.gatewayPort || GATEWAY_PORT;
}

/**
 * Run a JoyClaw agent command and collect stdout.
 * spawn(node, [openclaw, "agent", "--agent", "main", "--timeout", timeoutSec, "--json", "--message", message])
 */
export async function runAgent(
  message: string,
  timeoutSec = 300,
  config?: JoyClawBridgeConfig,
): Promise<string> {
  const nodePath = getNodePath(config);
  const openclawPath = getOpenclawPath(config);

  return new Promise<string>((resolve, reject) => {
    let child: ChildProcess;
    try {
      child = spawn(nodePath, [
        openclawPath,
        "agent",
        "--agent", "main",
        "--timeout", String(timeoutSec),
        "--json",
        "--message", message,
      ], {
        stdio: ["ignore", "pipe", "pipe"],
        timeout: (timeoutSec + 30) * 1000,
      });
    } catch (error) {
      reject(new Error(
        `Failed to spawn JoyClaw agent: ${error instanceof Error ? error.message : String(error)}`,
      ));
      return;
    }

    let stdout = "";
    let stderr = "";

    child.stdout?.on("data", (chunk: Buffer) => {
      stdout += chunk.toString();
    });
    child.stderr?.on("data", (chunk: Buffer) => {
      stderr += chunk.toString();
    });

    const timer = setTimeout(() => {
      child.kill("SIGTERM");
      reject(new Error(`JoyClaw agent timed out after ${timeoutSec}s`));
    }, (timeoutSec + 10) * 1000);

    child.on("error", (error) => {
      clearTimeout(timer);
      reject(new Error(`JoyClaw agent error: ${error.message}`));
    });

    child.on("close", (code) => {
      clearTimeout(timer);
      if (code !== 0) {
        reject(new Error(
          `JoyClaw agent exited with code ${code ?? "null"}. stderr: ${stderr.slice(0, 500)}`,
        ));
        return;
      }
      resolve(stdout);
    });
  });
}

/**
 * Parse agent JSON output to extract result text.
 * Expected format: { payloads: [{ text: "..." }] } at end of stdout.
 */
export function parseAgentOutput(stdout: string): string {
  // Find last JSON block in output
  const lines = stdout.split("\n");
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i]?.trim();
    if (!line || !line.startsWith("{")) continue;
    try {
      const parsed = JSON.parse(line) as {
        payloads?: Array<{ text?: string }>;
        result?: string;
      };
      if (parsed.payloads?.length) {
        return parsed.payloads.map((p) => p.text ?? "").join("\n");
      }
      if (parsed.result) {
        return parsed.result;
      }
    } catch {
      continue;
    }
  }
  // Fallback: return trimmed stdout
  return stdout.trim();
}

/**
 * Check if JoyClaw gateway is alive by checking the port.
 */
export async function ensureGatewayAlive(
  config?: JoyClawBridgeConfig,
): Promise<boolean> {
  const port = getGatewayPort(config);
  try {
    const response = await fetch(`http://127.0.0.1:${port}/health`, {
      signal: AbortSignal.timeout(5000),
    });
    return response.ok;
  } catch {
    // Fallback: try to check if port is in use via lsof
    try {
      const { execFile } = await import("node:child_process");
      const { promisify } = await import("node:util");
      const execFileAsync = promisify(execFile);
      const { stdout } = await execFileAsync("lsof", ["-i", `:${port}`], {
        timeout: 5000,
      });
      return stdout.includes("LISTEN");
    } catch {
      return false;
    }
  }
}
