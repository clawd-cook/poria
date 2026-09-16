# Package Inventory for Rust Migration

## 1. @poria/core

**Purpose:** Domain types, contracts (trait interfaces), pipeline state machine, event factory, gate evaluation, risk classification, multi-repo topological sort.

### Exported Types / Interfaces

| Type                                   | Fields                                                                                                                                                                                                                |
| -------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline`                             | id, demandId, demandCode, demandName?, status: PipelineStatus, rawLink, operator, hasRegressed, config: PipelineConfig, stages: Stage[], repos: RepoConfig[], createdAt, updatedAt                                    |
| `PipelineConfig`                       | gates: GateRule[], trdScope: string[], repos: RepoConfig[]                                                                                                                                                            |
| `PipelineStatus`                       | "created" \| "running" \| "waiting_merge" \| "blocked" \| "completed" \| "failed" \| "cancelled"                                                                                                                      |
| `Stage`                                | id?, pipelineId, name: StageEnum, status: StageStatus, skillId?, retryCount, maxRetries, input?, output?, gateResults?, issue?: StageIssue, rollback?: RollbackInstruction, agentSessionId?, startedAt?, completedAt? |
| `StageEnum`                            | "init" \| "review_prd" \| "design" \| "workspace" \| "dev" \| "cr" \| "deploy"                                                                                                                                        |
| `StageStatus`                          | "pending" \| "running" \| "completed" \| "failed" \| "blocked" \| "skipped"                                                                                                                                           |
| `StageIssue`                           | class, message, retryable                                                                                                                                                                                             |
| `StageOutput`                          | Record<string, unknown>                                                                                                                                                                                               |
| `SkillInput`                           | stage: Stage, pipeline: Pipeline, [key]: unknown                                                                                                                                                                      |
| `SkillOutput`                          | output: StageOutput, gatesPass?: boolean                                                                                                                                                                              |
| `DemandMetadata`                       | demandId, demandCode, name, status?, demandProjectId?, processor?: UserVO, proposer?: UserVO, receiver?: UserVO, prdUrl, attachments: CardAttachment[], rawLink                                                       |
| `UserVO`                               | erp, name, orgId?, orgName?                                                                                                                                                                                           |
| `CardAttachment`                       | tagName, name, url                                                                                                                                                                                                    |
| `RepoConfig`                           | name, gitUrl, branch, baseBranch, gitlabProjectPath, dependsOn?, buildCmd?                                                                                                                                            |
| `GateRule`                             | id, name, enabled, threshold, onFail: GateOnFail, gatePhase: GatePhase, regressTo?: StageEnum                                                                                                                         |
| `GateResult`                           | ruleId, pass, actual, threshold, message                                                                                                                                                                              |
| `GateEvaluation`                       | allPass, details: GateResult[], blockingFailures, warnFailures                                                                                                                                                        |
| `GatePhase`                            | "stage_exit" \| "deploy"                                                                                                                                                                                              |
| `GateOnFail`                           | "block" \| "warn" \| "regress"                                                                                                                                                                                        |
| `RollbackCommand`                      | type: "delete_branch" \| "close_mr" \| "revert_commit" \| "remove_worktree" \| "revert_mr", params: Record<string, string>                                                                                            |
| `RollbackInstruction`                  | stageIndex, commands: RollbackCommand[]                                                                                                                                                                               |
| `AgentTaskInput`                       | prompt, worktreePath, systemPrompt?, model?, maxBudgetUsd?, maxTurns?, timeoutMs?, extraTools?, onProgress?                                                                                                           |
| `AgentTaskResult`                      | success, result?, error?, sessionId?, costUsd?, messages: unknown[]                                                                                                                                                   |
| `IssuePolicy`                          | autoRetry, notifyRoles: string[], escalateAt, retryDelay?, note?                                                                                                                                                      |
| `PipelineEvent`                        | union of 27+ event types (pipeline_created, stage_started, agent_dispatched, gate_evaluated, human_assist_requested, git_commit, mr_created, worktree_created, rollback_executed, etc.)                               |
| `RiskLevel`                            | "low" \| "medium" \| "high" \| "critical"                                                                                                                                                                             |
| `TimeoutTier`                          | "GIT_SHORT" \| "GIT_MEDIUM" \| "GIT_LONG" \| "BUILD" \| "AGENT"                                                                                                                                                       |
| `StageResult`                          | crScore?, testCoverage?, ciBuildPass?, securityPass?, diffLines?, hasConflict?, [key]                                                                                                                                 |
| **Contracts (trait-like interfaces):** |                                                                                                                                                                                                                       |
| `CapabilityMetadata`                   | id, name, description, version                                                                                                                                                                                        |
| `IChannel<TInput, TOutput>`            | metadata, execute(input, ChannelContext)                                                                                                                                                                              |
| `IResource<TInput, TOutput>`           | metadata, execute(input, ResourceContext)                                                                                                                                                                             |
| `ICommand<TArgs, TResult>`             | metadata, execute(args, CommandContext)                                                                                                                                                                               |
| `ISkill`                               | metadata, execute(SkillInput, SkillContext)                                                                                                                                                                           |
| `ChannelContext`                       | credentials, pipelineId?                                                                                                                                                                                              |
| `ResourceContext`                      | pipelineId?, workdir?                                                                                                                                                                                                 |
| `CommandContext`                       | operator                                                                                                                                                                                                              |
| `SkillContext`                         | pipelineId, workdir, credentials                                                                                                                                                                                      |

### Exported Functions / Classes / Constants

| Export                                     | Description                                                      |
| ------------------------------------------ | ---------------------------------------------------------------- |
| `STAGE_ORDER`                              | Const array of 7 stage names in order                            |
| `TIMEOUT`                                  | Const object of timeout values (30s, 120s, 300s, 600s, 1800s)    |
| `IssueClass` (enum)                        | 15 issue classifications (COMPILATION_ERROR, TEST_FAILURE, etc.) |
| `ISSUE_POLICIES`                           | Record mapping IssueClass to retry/notify/escalation policy      |
| `InvalidTransitionError`                   | Error for invalid state transitions                              |
| `canPipelineTransition(from, to)`          | Check if pipeline status transition is valid                     |
| `canStageTransition(from, to)`             | Check if stage status transition is valid                        |
| `transitionPipeline(pipeline, to)`         | Mutate pipeline status (throws on invalid)                       |
| `transitionStage(stage, to)`               | Mutate stage status (throws on invalid)                          |
| 27 event factory functions                 | e.g. `pipelineCreatedEvent()`, `stageStartedEvent()`, etc.       |
| `crScoreMeetsThreshold(actual, threshold)` | Compare CR grade strings                                         |
| `evaluateGates(result, rules, phase)`      | Run gate rules and return GateEvaluation                         |
| `DEFAULT_GATES`                            | 6 default gate rules                                             |
| `createPipelineId()`                       | Generate `pl-YYYYMMDD-<nanoid(8)>`                               |
| `CircularDependencyError`                  | Error for cycles in repo dependency graph                        |
| `topologicalSort(repos)`                   | Topological sort of RepoConfig by dependsOn                      |
| `classifyRisk(changedFiles, diffLines)`    | Classify risk level from file patterns and diff size             |

### External Dependencies

- `nanoid` (for ID generation)

### Inter-package Dependencies

- None (leaf package)

---

## 2. @poria/resources

**Purpose:** Shell command execution, git worktree lifecycle, Claude Agent SDK pool with timeouts, agent output guard (scope/diff/dep checks), session tracker.

### Exported Types / Interfaces

| Type                   | Fields                                                                                       |
| ---------------------- | -------------------------------------------------------------------------------------------- |
| `TerminalExecInput`    | command, cwd?, env?: Record<string,string>, timeoutMs?                                       |
| `TerminalExecResult`   | code, stdout, stderr                                                                         |
| `WorktreeCreateInput`  | repo: RepoConfig, pipelineId, baseBranch                                                     |
| `WorktreeCreateResult` | worktreePath, branch                                                                         |
| `StageAgentConfig`     | allowedTools: string[], maxBudgetUsd, maxTurns, timeoutMs                                    |
| `IAgentSDK`            | query(prompt, AgentQueryOptions): AsyncIterable<SDKMessage>                                  |
| `AgentQueryOptions`    | cwd?, maxBudgetUsd?, maxTurns?, abortController?, model?, appendSystemPrompt?, allowedTools? |
| `SDKMessage`           | type, message?, result?, total_cost_usd?                                                     |
| `ViolationSeverity`    | "block" \| "warn"                                                                            |
| `ViolationType`        | "trd_scope_empty" \| "out_of_scope" \| "diff_too_large" \| "blocked_dependency"              |
| `Violation`            | type, severity, message, file?, actual?, threshold?, dependency?                             |
| `OutputGuardConfig`    | allowedPaths: string[], maxDiffLines, blockedDependencies: string[]                          |
| `AgentOutput`          | changedFiles: string[], totalDiffLines, addedDependencies: {name}[]                          |
| `GuardResult`          | pass, violations: Violation[]                                                                |

### Exported Classes / Functions

| Export               | Description                                                             |
| -------------------- | ----------------------------------------------------------------------- |
| `TimeoutError`       | Error thrown when shell command exceeds timeout                         |
| `TerminalResource`   | IResource impl: shell command execution via `child_process.spawn`       |
| `WorktreeResource`   | IResource impl: git worktree create/clean/remove                        |
| `ClaudeAgentPool`    | Manages Agent SDK dispatch with total+idle timeouts                     |
| `STAGE_AGENT_CONFIG` | Per-stage agent config (tools, budget, turns, timeout)                  |
| `OutputGuard`        | Validates agent output: file scope (minimatch), diff size, blocked deps |
| `SessionTracker`     | In-memory Map tracking (pipelineId, stageName) -> sessionId             |

### External Dependencies

- `minimatch` (glob matching for output guard)
- Node APIs: `child_process.spawn`, `fs.existsSync`, `path.join/resolve`

### Inter-package Dependencies

- `@poria/core` (types: IResource, CapabilityMetadata, ResourceContext, RepoConfig, AgentTaskInput, AgentTaskResult, StageEnum)

---

## 3. @poria/commands

**Purpose:** Pipeline execution engine (executor loop, stage dispatch, gate evaluation, CR regress), queue worker with MR polling, exception classifier, error handler with human-loop, rollback orchestrator.

### Exported Types / Interfaces

| Type                     | Fields                                                           |
| ------------------------ | ---------------------------------------------------------------- |
| `IPipelineStore`         | load(id), saveStageTx(stage, pipeline, events)                   |
| `ISkillLoader`           | load(skillId): ISkill                                            |
| `ICredentialGuard`       | ensureValid(): credentials                                       |
| `IMultiRepoOrchestrator` | execute(pipeline, stage, skill, credentials): SkillOutput        |
| `IHumanLoop`             | notify(pipeline, stage, issueClass)                              |
| `HandleErrorResult`      | issueClass: IssueClass, action: "retry" \| "blocked" \| "failed" |
| `IWorkerDeps`            | store, queue, executor, recovery, codingChannel, humanLoop       |
| `IFileLock`              | acquire(), release()                                             |
| `IRollbackDeps`          | store, terminal, codingChannel, jme, logger, fileExists          |

### Exported Classes / Functions

| Export                | Description                                                                                  |
| --------------------- | -------------------------------------------------------------------------------------------- |
| `PipelineExecutor`    | Main execution loop: iterates stages, dispatches skills, evaluates gates, handles CR regress |
| `PipelineWorker`      | Background worker: consumes queue + polls MR merge status                                    |
| `ExceptionClassifier` | Static classify(error) -> IssueClass based on message patterns                               |
| `handleStageError()`  | Error -> classify -> retry/block/fail based on ISSUE_POLICIES                                |
| `PipelineRollback`    | Rollback orchestrator: close MRs, delete branches, revert merged MRs, remove worktrees       |

### External Dependencies

- None (pure logic)

### Inter-package Dependencies

- `@poria/core` (Pipeline, Stage, events, state machine, gates, IssueClass, ISSUE_POLICIES, STAGE_ORDER, RollbackCommand)

---

## 4. @poria/skills

**Purpose:** 7 pipeline stage skill implementations (all currently fixture/stub), human-loop coordinator with fuzzy reply parsing, stage-to-skill mapping.

### Exported Classes / Functions

| Export                  | Description                                                        |
| ----------------------- | ------------------------------------------------------------------ |
| `InitSkill`             | ISkill: parse demand link, export PRD (fixture only)               |
| `ReviewPrdSkill`        | ISkill: analyze PRD, produce PRD_REVIEW.md (fixture only)          |
| `GenTrdSkill`           | ISkill: generate TRD.md, extract allowed scope (fixture only)      |
| `WorkspaceSkill`        | ISkill: create git worktree, bind branch (fixture only)            |
| `GenCodeSkill`          | ISkill: agent-driven code generation (fixture only)                |
| `CodeReviewSkill`       | ISkill: agent-driven code review + security scan (fixture only)    |
| `DeploySkill`           | ISkill: build, push, create MR (fixture only)                      |
| `HumanLoopCoordinator`  | IHumanLoop impl: notify/renotify/escalate/pollReply (fixture only) |
| `parseHumanReply(text)` | Fuzzy match Chinese/English patterns to resume/skip/cancel         |
| `STAGE_SKILL_MAP`       | Record<StageEnum, skillId> mapping                                 |
| `isFixtureMode()`       | Check PORIA_PIPELINE_FIXTURE env var                               |

### External Dependencies

- None (pure logic + env check)

### Inter-package Dependencies

- `@poria/core` (ISkill, SkillContext, SkillInput, SkillOutput, CapabilityMetadata, Pipeline, Stage, StageEnum)

---

## 5. @poria/infrastructure

**Purpose:** SQLite persistence (pipeline store, event store, audit store, queue, recovery, archiver, backup), auth (credentials CRUD, browser cookie extraction, credential guard), config, logger, metrics, plugin loader.

### Exported Types / Interfaces

| Type                 | Fields                                                                                                                              |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `AuditAction`        | "git_commit" \| "git_push" \| "mr_create" \| "mr_merge" \| "branch_delete"                                                          |
| `AuditEntry`         | pipelineId, stage?, action, operator, detail                                                                                        |
| `IWorktreeCleaner`   | cleanDirtyState(repoName, pipelineId)                                                                                               |
| `IHumanLoopNotifier` | renotify(pipeline, stage)                                                                                                           |
| `RecoveryResult`     | runningRecovered, blockedRenotified, errors[]                                                                                       |
| `ArchiveResult`      | archivedCount                                                                                                                       |
| `JacpCredentials`    | username, cookie                                                                                                                    |
| `AuthStatus`         | loggedIn, username?                                                                                                                 |
| `BrowserCookie`      | name?, value?, domain?, path?, secure?, expires?                                                                                    |
| `ApiProbe`           | (credentials) => Promise<void>                                                                                                      |
| `PoriaConfig`        | gates{crScore,testCov,diffSize}, timeouts{gitShort..agent}, retry{maxRetries..mrTimeout}, paths{dbPath,archiveDir,backupDir,logDir} |
| `LogLevel`           | "debug" \| "info" \| "warn" \| "error"                                                                                              |
| `LogContext`         | pipelineId?, stageName?, [key]                                                                                                      |
| `ILogger`            | debug/info/warn/error(msg, ctx), child(ctx)                                                                                         |
| `IMetricsCollector`  | 17 record methods + snapshot()                                                                                                      |
| `MetricEntry`        | name, type, labels, value, observations?                                                                                            |
| `MetricsSnapshot`    | timestamp, metrics[]                                                                                                                |
| `IMetricsReporter`   | report(snapshot)                                                                                                                    |
| `IPluginLoader`      | load<T>(id), register(id, instance), has(id)                                                                                        |

### Exported Classes / Functions

| Export                           | Description                                               |
| -------------------------------- | --------------------------------------------------------- |
| `initDatabase(dbPath)`           | Create/migrate SQLite DB with better-sqlite3              |
| `SqlitePipelineStore`            | Pipeline/stage CRUD, atomic saveStageTx, findByStatus     |
| `EventStore`                     | Read-only event queries by pipeline/kind                  |
| `AuditStore`                     | Append-only audit log, query by pipeline                  |
| `PipelineQueue`                  | SQLite-backed priority queue with atomic dequeue          |
| `WorkerLock`                     | File-based PID lock for single-instance worker            |
| `PipelineRecovery`               | Recover running pipelines on restart, re-notify blocked   |
| `EventArchiver`                  | Archive old events to JSONL files, delete from SQLite     |
| `EventReplayService`             | Replay events from SQLite or JSONL archives               |
| `DatabaseBackup`                 | SQLite backup API, cleanup old backups                    |
| `getCredentials()`               | Read ~/.poria/auth.json                                   |
| `saveCredentials()`              | Atomic write to ~/.poria/auth.json                        |
| `getStatus()`                    | Check login status                                        |
| `logout()`                       | Remove auth file                                          |
| `redactCookie()`                 | Safe logging of cookie values                             |
| `authHeaders()`                  | Build Cookie header from credentials                      |
| `parseUsernameFromCookie()`      | Extract erp_erp value from cookie string                  |
| `assertSafeAuthRoot()`           | Prevent writing auth to project dirs                      |
| `readBrowserCookies()`           | Dynamic import @rookie-rs/api to read Chrome cookies      |
| `CredentialGuard`                | Probe+refresh flow: validate -> auto-refresh from browser |
| `AuthExpiredDuringPipelineError` | Error for unrecoverable auth expiry                       |
| `loadConfig()`                   | Deep merge defaults + file + env overrides                |
| `createLogger() / JsonLogger`    | JSON-structured stderr logger with levels                 |
| `InMemoryMetricsCollector`       | Counters + histograms in memory                           |
| `StdoutReporter`                 | Print metrics to stdout                                   |
| `FileReporter`                   | Append metrics as JSONL to file                           |
| `PluginLoader`                   | In-memory capability registry (register/load/has)         |

### External Dependencies

- `better-sqlite3` (SQLite with WAL, transactions, backup API)
- `@rookie-rs/api` (optional, dynamic import for browser cookie extraction)
- Node APIs: `fs`, `path`, `os`, `readline`, `process.pid`, `process.kill`, `process.env`

### Inter-package Dependencies

- `@poria/core` (Pipeline, Stage, PipelineEvent, PipelineStatus, StageEnum, StageStatus, StageIssue, PipelineConfig, RollbackInstruction, STAGE_ORDER, worktreeCleanedEvent, stageFailedEvent)

---

## 6. @poria/channel-coding

**Purpose:** EasyCI repository search (GQL), branch listing (GQL), MR create/get-status/find (GitLab REST API), git URL normalization helpers, fixture mode.

### Exported Types / Interfaces

| Type                  | Fields                                                        |
| --------------------- | ------------------------------------------------------------- |
| `JacpCredentials`     | cookie, username                                              |
| `CodingRepo`          | code, defaultBranchName?, gitUrl, homeUrl?, repoLabel?        |
| `CodingBranch`        | name, status?                                                 |
| `MrStatus`            | "opened" \| "closed" \| "merged" \| "locked"                  |
| `MrInfo`              | url, iid, sourceBranch, targetBranch, state: MrStatus         |
| `FindMrQuery`         | projectPath, sourceBranch, targetBranch, state?               |
| `CreateMrInput`       | title, description?, sourceBranch?, targetBranch?, projectId? |
| `CreateMrResult`      | url, iid?, sourceBranch, targetBranch                         |
| `CodingChannelInput`  | action + action-specific fields                               |
| `CodingChannelOutput` | discriminated union by action                                 |

### Exported Classes / Functions

| Export                        | Description                                                                                           |
| ----------------------------- | ----------------------------------------------------------------------------------------------------- |
| `createCodingChannel()`       | Factory for CodingChannelImpl (IChannel)                                                              |
| `queryAllRepos()`             | EasyCI GQL: search repos by name                                                                      |
| `queryBranches()`             | EasyCI GQL: list branches for a repo                                                                  |
| `createMergeRequestLive()`    | GitLab REST: POST merge request                                                                       |
| `getMrStatusLive()`           | GitLab REST: GET MR state                                                                             |
| `findMrLive()`                | GitLab REST: search for existing MR                                                                   |
| `normalizeGitUrl()`           | Normalize SSH/HTTPS git URLs to lowercase HTTPS                                                       |
| `repoNameFromGitUrl()`        | Extract repo name from git URL                                                                        |
| `repoSearchPathFromGitUrl()`  | Extract path portion for search                                                                       |
| `sameGitUrl()`                | Compare normalized git URLs                                                                           |
| `getOriginGitUrl()`           | Run `git remote get-url origin`                                                                       |
| `projectIdFromGitUrl()`       | Extract GitLab project path from git URL                                                              |
| `detectCurrentBranch()`       | Run `git branch --show-current`                                                                       |
| `detectDefaultTargetBranch()` | Run `git symbolic-ref refs/remotes/origin/HEAD`                                                       |
| Fixture functions             | searchReposFixture, listBranchesFixture, createMergeRequestFixture, getMrStatusFixture, findMrFixture |

### External Dependencies

- Node APIs: `child_process.execFile`, `util.promisify`, `process.cwd`, `process.env`
- `fetch` (global, HTTP calls to EasyCI GQL + GitLab REST)

### Inter-package Dependencies

- `@poria/core` (IChannel, CapabilityMetadata, ChannelContext)

---

## 7. @poria/channel-xingyun

**Purpose:** Xingyun demand management (get demand, list card attachments, resolve PRD link, bind branch via EasyCI change, communicate/accept demand status), with JACP API client, git operations, URL parsing.

### Exported Types / Interfaces

| Type                   | Fields                                                                                                        |
| ---------------------- | ------------------------------------------------------------------------------------------------------------- |
| `JacpCredentials`      | cookie, username                                                                                              |
| `DemandDetail`         | id, demandCode, name, status?, demandDesc?, projectId?, processor/proposer/receiver?: UserVO, extendedFields? |
| `UserVO`               | erp?, name?, orgId?, orgName?                                                                                 |
| `DemandActionResult`   | demandId?, demandStatusCode?, taskOwner?, taskId?, taskName?                                                  |
| `CardAttachment`       | id?, tagId?, tagName, name, url                                                                               |
| `EasyciDeployApp`      | id, name, appKey, systemId?, systemKey?, gitUrl?, deploySystemName?, alreadyBound                             |
| `BindDeployAppsResult` | devSpaceId?, records: EasyciDeployApp[]                                                                       |
| `CreateChangeInput`    | devSpaceId, name?, code, issueCode, branch, branchOperateType, bindApps[]                                     |
| `LocalGitContext`      | workspacePath?, gitUrl?, repoName?, currentBranch?                                                            |
| `XingyunChannelInput`  | action + action-specific fields                                                                               |
| `XingyunChannelOutput` | discriminated union by action                                                                                 |

### Exported Classes / Functions

| Export                                                                           | Description                                                               |
| -------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| `createXingyunChannel()`                                                         | Factory for XingyunChannelImpl (IChannel)                                 |
| `parseXingyunDemandUrl()`                                                        | Parse xingyun demand URL to { demandId, demandCode, url }                 |
| `featureBranchName()`                                                            | Generate `feature_<code>` branch name                                     |
| `featureSlug()`                                                                  | Generate `feat-<slug>` for task naming                                    |
| `resolvePrdFromAttachments()`                                                    | Select single JoySpace PRD from card attachments                          |
| `PrdResolveError`                                                                | Error for NO_PRD or AMBIGUOUS_PRD                                         |
| `isDemandActive()`                                                               | Check demand status (MVP: always true)                                    |
| `isJoySpacePrdLink()`                                                            | Check if URL is joyspace.jd.com                                           |
| `parsePrdLink()`                                                                 | Parse PRD URL to { url, demandId?, fingerprint }                          |
| JACP API: `getDemandById()`, `communicateDemand()`, `acceptDemand()`             | Demand CRUD via JACP REST                                                 |
| JACP API: `getCardByCode()`, `getCardById()`                                     | Card queries                                                              |
| JACP API: `getSpaceById()`                                                       | Space queries                                                             |
| JACP API: `queryBindDeployApps()`, `createChange()`                              | EasyCI GQL for branch binding                                             |
| JACP API: `fetchJsonEnvelope()`, `jacpFetch()`, `fetchWithRetry()`               | HTTP client with retry, auth check, envelope unwrap                       |
| Git: `getLocalGitContext()`, `createAndPushBranch()`, `detectOriginHeadBranch()` | Local git operations                                                      |
| Fixtures                                                                         | FIXTURE_DEMAND, FIXTURE_ATTACHMENTS, FIXTURE_BIND, isXingyunFixtureMode() |

### External Dependencies

- Node APIs: `child_process.execFile`, `util.promisify`, `process.cwd`, `process.env`
- `fetch` (global, HTTP calls to JACP + EasyCI)

### Inter-package Dependencies

- `@poria/core` (IChannel, CapabilityMetadata, ChannelContext)

---

## 8. @poria/channel-jme

**Purpose:** JingME messaging via JoyClaw bridge (spawn external Node process), reply parsing for human-loop, gateway health check.

### Exported Types / Interfaces

| Type                  | Fields                                                                           |
| --------------------- | -------------------------------------------------------------------------------- |
| `JoyClawBridgeConfig` | nodePath?, openclawPath?, gatewayPort?                                           |
| `ReplyAction`         | "resume" \| "skip" \| "cancel" \| "unknown"                                      |
| `JmeChannelInput`     | action: "send" \| "readReplies" \| "ensureGatewayAlive" + action-specific fields |
| `JmeChannelOutput`    | discriminated union by action                                                    |

### Exported Classes / Functions

| Export                 | Description                                                                              |
| ---------------------- | ---------------------------------------------------------------------------------------- |
| `createJmeChannel()`   | Factory for JmeChannelImpl (IChannel)                                                    |
| `runAgent()`           | Spawn JoyClaw agent process, collect stdout                                              |
| `parseAgentOutput()`   | Extract result from JoyClaw JSON output                                                  |
| `ensureGatewayAlive()` | Health check via HTTP + lsof fallback                                                    |
| `parseReply()`         | Fuzzy match Chinese/English patterns to action                                           |
| Fixture functions      | sendFixture, readRepliesFixture, ensureGatewayAliveFixture, get/clearFixtureSentMessages |

### External Dependencies

- Node APIs: `child_process.spawn/execFile`, `os.homedir`, `path`, `fetch`

### Inter-package Dependencies

- `@poria/core` (IChannel, CapabilityMetadata, ChannelContext)

---

## 9. @poria/channel-joyspace

**Purpose:** Export JoySpace documents to Markdown, including diagram rendering (drawio -> SVG/mermaid), with vendor modules for API client and content conversion.

### Exported Types / Interfaces

| Type                    | Fields                                                                         |
| ----------------------- | ------------------------------------------------------------------------------ |
| `JacpCredentials`       | cookie, username                                                               |
| `Logger`                | info?, warn?, error?, debug?                                                   |
| `ExportJoySpaceInput`   | url, outputDir?, outputName?, tenantCode?, credentials?, logger?               |
| `ExportJoySpaceResult`  | outputPath, title, cookieSource?                                               |
| `JoySpaceChannelInput`  | action: "exportToMarkdown", url, outputDir?, outputName?, tenantCode?, logger? |
| `JoySpaceChannelOutput` | action: "exportToMarkdown", result: ExportJoySpaceResult                       |

### Exported Classes / Functions

| Export                     | Description                                                                                              |
| -------------------------- | -------------------------------------------------------------------------------------------------------- |
| `createJoySpaceChannel()`  | Factory for JoySpaceChannelImpl (IChannel)                                                               |
| `exportJoySpaceMarkdown()` | Full export pipeline: fetch page -> fetch content -> fetch diagrams -> convert to markdown -> write file |
| `isJoySpaceFixtureMode()`  | Check PORIA_JOYSPACE_FIXTURE env var                                                                     |

### External Dependencies

- Node APIs: `fs`, `path`, `fetch`
- Vendor modules (bundled .mjs): `joyspace-api-client.mjs`, `joyspace-content-to-markdown.mjs`, `joyspace-drawio-to-mermaid.mjs`

### Inter-package Dependencies

- `@poria/core` (IChannel, CapabilityMetadata, ChannelContext)

---

## 10. @poria/channel-defect

**Purpose:** Placeholder for defect management integration (P1 future).

### Exports

- `DEFECT_CHANNEL_PLACEHOLDER = true`

### Dependencies

- None

---

## Summary: Node.js API Usage Across All Packages

| Node API                                                       | Used By                                                                            | Rust Equivalent                       |
| -------------------------------------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------- |
| `child_process.spawn`                                          | resources/terminal, channels/jme                                                   | `tokio::process::Command`             |
| `child_process.execFile`                                       | channels/coding, channels/xingyun, channels/jme                                    | `tokio::process::Command`             |
| `fs` (read/write/exists/mkdir/unlink/readdir/createReadStream) | infrastructure (store, auth, config, archiver, backup, metrics), channels/joyspace | `std::fs`, `tokio::fs`                |
| `path` (join, resolve, dirname, basename, parse)               | infrastructure, resources, channels                                                | `std::path::PathBuf`                  |
| `os.homedir()`                                                 | infrastructure/auth, channels/jme                                                  | `dirs::home_dir()`                    |
| `readline`                                                     | infrastructure/archiver (JSONL reading)                                            | `tokio::io::BufReader`                |
| `process.env`                                                  | infrastructure/config, channels (fixture modes, base URLs)                         | `std::env::var()`                     |
| `process.pid`                                                  | infrastructure/auth (atomic write), queue (lock)                                   | `std::process::id()`                  |
| `process.kill(pid, 0)`                                         | infrastructure/queue (process alive check)                                         | `nix::sys::signal::kill`              |
| `process.cwd()`                                                | channels/coding, xingyun, joyspace                                                 | `std::env::current_dir()`             |
| `process.stderr.write`                                         | infrastructure/logger                                                              | `eprintln!` or `tracing`              |
| `process.stdout.write`                                         | infrastructure/metrics stdout reporter                                             | `println!`                            |
| `fetch` (global)                                               | channels/coding, xingyun, jme, joyspace                                            | `reqwest`                             |
| `setTimeout/setInterval/clearTimeout`                          | resources (agent pool timeouts), commands (worker polling)                         | `tokio::time`                         |
| `AbortController`                                              | resources/agent-pool, xingyun/client                                               | `tokio_util::sync::CancellationToken` |
| `URL` (global)                                                 | channels/xingyun (demand URL parsing), joyspace                                    | `url::Url`                            |
| `structuredClone`                                              | infrastructure/config                                                              | `.clone()`                            |
| Dynamic import                                                 | infrastructure/auth (rookie-rs), xingyun (easyci-repo)                             | compile-time features / optional deps |

## Summary: External npm Dependencies

| Package          | Used By                        | Rust Equivalent                       |
| ---------------- | ------------------------------ | ------------------------------------- |
| `nanoid`         | core                           | `nanoid` crate or `uuid`              |
| `minimatch`      | resources/output-guard         | `glob` or `globset` crate             |
| `better-sqlite3` | infrastructure                 | `rusqlite` (already in Tauri backend) |
| `@rookie-rs/api` | infrastructure/auth (optional) | Native Rust — the crate IS Rust       |

## Dependency Graph

```
core (leaf)
  ├── resources (depends on core)
  ├── commands (depends on core)
  ├── skills (depends on core)
  ├── infrastructure (depends on core)
  ├── channel-coding (depends on core)
  ├── channel-xingyun (depends on core)
  ├── channel-jme (depends on core)
  ├── channel-joyspace (depends on core)
  └── channel-defect (no deps)
```

All packages depend only on `@poria/core`. There are no inter-package dependencies beyond that (channels are independent, infrastructure is independent, etc.). The `commands` package defines trait-like interfaces that infrastructure and resources implement at the composition root.
