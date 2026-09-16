import { describe, it, expect } from "vitest";
import { IssueClass } from "@poria/core";
import { ExceptionClassifier } from "../exception-classifier.js";

describe("ExceptionClassifier", () => {
  it("classifies compilation errors", () => {
    expect(ExceptionClassifier.classify(new Error("compilation failed"))).toBe(IssueClass.COMPILATION_ERROR);
    expect(ExceptionClassifier.classify(new Error("build failed"))).toBe(IssueClass.COMPILATION_ERROR);
    expect(ExceptionClassifier.classify(new Error("tsc error TS2345"))).toBe(IssueClass.COMPILATION_ERROR);
  });

  it("classifies test failures", () => {
    expect(ExceptionClassifier.classify(new Error("test fail in suite"))).toBe(IssueClass.TEST_FAILURE);
    expect(ExceptionClassifier.classify(new Error("vitest exited with code 1"))).toBe(IssueClass.TEST_FAILURE);
    expect(ExceptionClassifier.classify(new Error("jest failed"))).toBe(IssueClass.TEST_FAILURE);
  });

  it("classifies agent timeout", () => {
    expect(ExceptionClassifier.classify(new Error("AGENT_TIMEOUT exceeded"))).toBe(IssueClass.AGENT_TIMEOUT);
    expect(ExceptionClassifier.classify(new Error("timeout waiting for agent"))).toBe(IssueClass.AGENT_TIMEOUT);
  });

  it("classifies LLM rate limit", () => {
    expect(ExceptionClassifier.classify(new Error("rate limit exceeded"))).toBe(IssueClass.LLM_RATE_LIMIT);
    expect(ExceptionClassifier.classify(new Error("429 Too Many Requests"))).toBe(IssueClass.LLM_RATE_LIMIT);
    expect(ExceptionClassifier.classify(new Error("overloaded"))).toBe(IssueClass.LLM_RATE_LIMIT);
  });

  it("classifies requirement ambiguity", () => {
    expect(ExceptionClassifier.classify(new Error("ambiguous requirement"))).toBe(IssueClass.REQUIREMENT_AMBIG);
    expect(ExceptionClassifier.classify(new Error("P0 unanswered"))).toBe(IssueClass.REQUIREMENT_AMBIG);
  });

  it("classifies PRD invalid", () => {
    expect(ExceptionClassifier.classify(new Error("PRD empty"))).toBe(IssueClass.PRD_INVALID);
    expect(ExceptionClassifier.classify(new Error("PRD export failed"))).toBe(IssueClass.PRD_INVALID);
    expect(ExceptionClassifier.classify(new Error("PRD invalid format"))).toBe(IssueClass.PRD_INVALID);
  });

  it("classifies merge conflict", () => {
    expect(ExceptionClassifier.classify(new Error("merge conflict detected"))).toBe(IssueClass.MERGE_CONFLICT);
    expect(ExceptionClassifier.classify(new Error("CONFLICT in src/file.ts"))).toBe(IssueClass.MERGE_CONFLICT);
  });

  it("classifies low CR score", () => {
    expect(ExceptionClassifier.classify(new Error("cr score below threshold"))).toBe(IssueClass.LOW_CR_SCORE);
    expect(ExceptionClassifier.classify(new Error("LOW_CR_SCORE"))).toBe(IssueClass.LOW_CR_SCORE);
  });

  it("classifies diff too large", () => {
    expect(ExceptionClassifier.classify(new Error("diff too large"))).toBe(IssueClass.DIFF_TOO_LARGE);
    expect(ExceptionClassifier.classify(new Error("DIFF_TOO_LARGE"))).toBe(IssueClass.DIFF_TOO_LARGE);
  });

  it("classifies permission denied", () => {
    expect(ExceptionClassifier.classify(new Error("permission denied"))).toBe(IssueClass.PERMISSION_DENIED);
    expect(ExceptionClassifier.classify(new Error("403 Forbidden"))).toBe(IssueClass.PERMISSION_DENIED);
    expect(ExceptionClassifier.classify(new Error("forbidden action"))).toBe(IssueClass.PERMISSION_DENIED);
  });

  it("classifies infra failure", () => {
    expect(ExceptionClassifier.classify(new Error("ECONNREFUSED 127.0.0.1:3000"))).toBe(IssueClass.INFRA_FAILURE);
    expect(ExceptionClassifier.classify(new Error("ENOTFOUND api.example.com"))).toBe(IssueClass.INFRA_FAILURE);
  });

  it("classifies security violation", () => {
    expect(ExceptionClassifier.classify(new Error("security scan failed"))).toBe(IssueClass.SECURITY_VIOLATION);
    expect(ExceptionClassifier.classify(new Error("blocked_dependency detected"))).toBe(IssueClass.SECURITY_VIOLATION);
  });

  it("classifies out of scope", () => {
    expect(ExceptionClassifier.classify(new Error("out_of_scope change detected"))).toBe(IssueClass.OUT_OF_SCOPE);
    expect(ExceptionClassifier.classify(new Error("OutputGuardError: file not in scope"))).toBe(IssueClass.OUT_OF_SCOPE);
  });

  it("classifies auth expired", () => {
    expect(ExceptionClassifier.classify(new Error("auth expired"))).toBe(IssueClass.AUTH_EXPIRED);
    expect(ExceptionClassifier.classify(new Error("cookie expired"))).toBe(IssueClass.AUTH_EXPIRED);
    expect(ExceptionClassifier.classify(new Error("AuthExpired"))).toBe(IssueClass.AUTH_EXPIRED);
  });

  it("falls back to UNKNOWN for unrecognized errors", () => {
    expect(ExceptionClassifier.classify(new Error("something unexpected"))).toBe(IssueClass.UNKNOWN);
    expect(ExceptionClassifier.classify("string error")).toBe(IssueClass.UNKNOWN);
    expect(ExceptionClassifier.classify(42)).toBe(IssueClass.UNKNOWN);
  });
});
