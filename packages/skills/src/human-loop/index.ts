import type { Pipeline, Stage } from "@poria/core";
import { isFixtureMode } from "../fixture.js";

export interface HumanReply {
  action: "resume" | "skip" | "cancel";
  rawMessage: string;
}

export interface IHumanLoop {
  notify(pipeline: Pipeline, stage: Stage, issueClass: string): Promise<void>;
  renotify(pipeline: Pipeline, stage: Stage): Promise<void>;
  escalate(pipeline: Pipeline, message: string): Promise<void>;
  pollReply(pipeline: Pipeline): Promise<HumanReply | null>;
}

const RESUME_PATTERNS = [/修复/, /fix/i, /已修复/, /已解决/, /重试/, /retry/i, /resume/i];
const SKIP_PATTERNS = [/跳过/, /skip/i, /忽略/, /ignore/i];
const CANCEL_PATTERNS = [/取消/, /cancel/i, /终止/, /abort/i, /stop/i];

export function parseHumanReply(text: string): HumanReply | null {
  const trimmed = text.trim();
  if (!trimmed) return null;

  for (const p of CANCEL_PATTERNS) {
    if (p.test(trimmed)) return { action: "cancel", rawMessage: trimmed };
  }
  for (const p of SKIP_PATTERNS) {
    if (p.test(trimmed)) return { action: "skip", rawMessage: trimmed };
  }
  for (const p of RESUME_PATTERNS) {
    if (p.test(trimmed)) return { action: "resume", rawMessage: trimmed };
  }

  return null;
}

export class HumanLoopCoordinator implements IHumanLoop {
  private readonly fixture: boolean;

  constructor() {
    this.fixture = isFixtureMode();
  }

  async notify(
    _pipeline: Pipeline,
    _stage: Stage,
    _issueClass: string,
  ): Promise<void> {
    if (this.fixture) return;
    throw new Error("HumanLoopCoordinator.notify: real implementation not yet available");
  }

  async renotify(_pipeline: Pipeline, _stage: Stage): Promise<void> {
    if (this.fixture) return;
    throw new Error("HumanLoopCoordinator.renotify: real implementation not yet available");
  }

  async escalate(_pipeline: Pipeline, _message: string): Promise<void> {
    if (this.fixture) return;
    throw new Error("HumanLoopCoordinator.escalate: real implementation not yet available");
  }

  async pollReply(_pipeline: Pipeline): Promise<HumanReply | null> {
    if (this.fixture) return null;
    throw new Error("HumanLoopCoordinator.pollReply: real implementation not yet available");
  }
}
