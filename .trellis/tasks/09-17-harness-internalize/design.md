# Design: Internalize harness-o2o-fe into Poria

## Overview

基于现有 Poria 架构，需要填充 4 个主要缺口来实现 harness 能力内化：

1. **AgentSdk 实现** — 将 `AgentSdk` trait 连接到真实的 Claude CLI subprocess
2. **Skill 实现** — 将 7 个 `NotImplemented` skill 连接到 Claude agent pool + 内置 skill prompt
3. **Feature Context** — 管理每个需求的阶段产出文件
4. **前端 Workflow UI** — 阶段展示、流式输出、人工审批

## Architecture Layers

```
┌─────────────────────────────────────────────────────────┐
│  Frontend (React)                                       │
│  WorkflowPanel → StageProgress → StreamOutput           │
│                → HumanLoopCard → FeatureFiles            │
├─────────────────────────────────────────────────────────┤
│  Tauri IPC Commands                                     │
│  submitPipeline / humanLoopRespond / getStreamOutput    │
├─────────────────────────────────────────────────────────┤
│  poria-commands: PipelineExecutor                       │
│  stage loop → SkillLoader → Skill.execute()             │
├─────────────────────────────────────────────────────────┤
│  poria-skills: Stage Skill Implementations              │
│  ReviewPrdSkill / GenTrdSkill / GenCodeSkill / CrSkill  │
│  ↕ construct prompt from skill template + feature files │
├─────────────────────────────────────────────────────────┤
│  poria-resources: ClaudeAgentPool                       │
│  AgentSdk(ClaudeCliSdk) → spawn `claude` CLI process    │
│  SessionTracker / OutputGuard                           │
├─────────────────────────────────────────────────────────┤
│  poria-core: Pipeline Engine                            │
│  StateMachine / Gates / Events / FeatureContext          │
├─────────────────────────────────────────────────────────┤
│  poria-channels: External I/O (no AI)                   │
│  JoySpace / Coding / Xingyun / JME / Defect             │
└─────────────────────────────────────────────────────────┘
```

## Component Design

### 1. ClaudeCliSdk (`poria-resources/src/claude/cli_sdk.rs`)

实现 `AgentSdk` trait，通过 subprocess 调用本机 `claude` CLI。

```rust
pub struct ClaudeCliSdk {
    claude_path: PathBuf,  // 通常是 `claude` (PATH resolve)
}

#[async_trait]
impl AgentSdk for ClaudeCliSdk {
    async fn query(
        &self,
        prompt: &str,
        options: AgentQueryOptions,
    ) -> Result<(Vec<SdkMessage>, Option<String>), AgentSdkError> {
        // 1. 构建 CLI 参数
        let mut cmd = Command::new(&self.claude_path);
        cmd.arg("-p").arg(prompt);
        
        if let Some(sys) = &options.append_system_prompt {
            cmd.arg("--system-prompt").arg(sys);
        }
        if let Some(tools) = &options.allowed_tools {
            cmd.arg("--allowedTools").arg(tools.join(","));
        }
        if let Some(cwd) = &options.cwd {
            cmd.current_dir(cwd);
        }
        
        cmd.arg("--output-format").arg("stream-json");
        cmd.arg("--max-turns").arg(options.max_turns.to_string());
        
        // 2. Spawn + 流式读取 stdout
        // 3. 解析 stream-json 事件，emit 到 event system
        // 4. 收集最终结果
    }
}
```

**Stream JSON 解析**: Claude CLI 的 `stream-json` 格式输出每行一个 JSON 事件。关键事件类型：
- `assistant` — Claude 文本输出（增量）
- `tool_use` — 工具调用
- `tool_result` — 工具结果
- `result` — 最终结果（含 session_id）

**进程管理**:
- `tokio::process::Command` 异步 spawn
- stdout 逐行读取，解析 JSON 事件
- stderr 捕获错误信息
- 通过 `tokio::select!` 实现超时控制（已有 `StageAgentConfig.timeout`）
- 通过 `child.kill()` 实现取消

**并发**: `ClaudeAgentPool` 已有 `tokio::Semaphore` 控制并发数，无需额外处理。

### 2. Skill Prompt Templates (`poria-skills/src/prompts/`)

每个 AI 阶段对应一个 prompt template 文件，从 harness skill 迁移并精简。

```
poria-skills/src/prompts/
├── prd_review.md      # 从 hfe-review-prd 迁移
├── trd_gen.md         # 从 hfe-gen-trd 迁移
├── code_impl.md       # 从 hfe-execute 迁移
└── cr_web.md          # 从 hfe-cr-web 迁移
```

**嵌入方式**: 使用 `include_str!()` 在编译时嵌入到 binary 中。

```rust
const PRD_REVIEW_PROMPT: &str = include_str!("prompts/prd_review.md");
const TRD_GEN_PROMPT: &str = include_str!("prompts/trd_gen.md");
const CODE_IMPL_PROMPT: &str = include_str!("prompts/code_impl.md");
const CR_WEB_PROMPT: &str = include_str!("prompts/cr_web.md");
```

**模板变量替换**: 简单字符串替换（不引入模板引擎）。

```rust
fn render_prompt(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}
```

常用变量:
- `{{feature_dir}}` — feature context 绝对路径
- `{{project_root}}` — 项目根目录
- `{{prd_content}}` — PRD 文件内容
- `{{prd_review_content}}` — PRD_REVIEW.md 内容
- `{{trd_content}}` — TRD.md 内容
- `{{git_diff}}` — 代码变更 diff

### 3. Skill Implementations (填充现有 `NotImplemented`)

每个 skill 的实现模式相同：

```rust
// 以 ReviewPrdSkill 为例
#[async_trait]
impl Skill for ReviewPrdSkill {
    async fn execute(&self, input: SkillInput, ctx: SkillContext) -> Result<SkillOutput> {
        // 1. 从 input 提取 feature_dir, prd_path
        let feature_dir = input.extra.get("feature_dir");
        let prd_content = fs::read_to_string(prd_path).await?;
        
        // 2. 渲染 prompt template
        let vars = hashmap! {
            "feature_dir" => feature_dir,
            "project_root" => ctx.workdir,
            "prd_content" => prd_content,
        };
        let system_prompt = render_prompt(PRD_REVIEW_PROMPT, &vars);
        
        // 3. 构建 agent task input
        let agent_input = AgentTaskInput {
            prompt: "分析这份 PRD，从前端视角进行需求澄清，生成 PRD_REVIEW.md".into(),
            options: AgentQueryOptions {
                cwd: Some(ctx.workdir.clone()),
                append_system_prompt: Some(system_prompt),
                allowed_tools: Some(stage_config.tools.clone()),
                max_turns: Some(stage_config.max_turns),
                ..Default::default()
            },
        };
        
        // 4. 调用 Claude agent pool
        let result = self.agent_pool.dispatch(agent_input).await?;
        
        // 5. 检查输出文件是否生成
        let review_path = feature_dir.join("PRD_REVIEW.md");
        let gates_pass = review_path.exists();
        
        Ok(SkillOutput {
            output: json!({ "prd_review_path": review_path }),
            gates_pass: Some(gates_pass),
        })
    }
}
```

**各 Skill 差异点**:

| Skill | 特殊逻辑 |
|-------|----------|
| ReviewPrdSkill | 输出后需检查 P0 问题数量，设置 gate |
| GenTrdSkill | gate 检查: PRD_REVIEW.md P0 已全部回答 |
| GenCodeSkill | 分发 SubAgent（多个 Claude 进程），OutputGuard 校验 |
| CodeReviewSkill | 需要先生成 git diff，传入 Claude |

**GenCodeSkill 的 SubAgent 编排**:
- 主 Claude 会话生成 TASK.md（执行计划）
- 人工审批 TASK.md
- 按 task 依赖顺序分发编码 SubAgent（每个 T-n 一个 Claude 进程）
- 这里的"SubAgent"不是 Claude 内部的 agent，而是 Poria 层面的多进程并发
- `ClaudeAgentPool` 的 semaphore 控制并发数

### 4. Feature Context (`poria-core/src/feature_context.rs`)

管理每个需求的阶段产出文件。

```rust
pub struct FeatureContext {
    pub id: String,              // feat-001-batch-export
    pub pipeline_id: String,     // 关联的 pipeline
    pub root: PathBuf,           // .poria/features/feat-001-batch-export/
    pub created_at: DateTime<Utc>,
}

impl FeatureContext {
    pub fn artifact_path(&self, name: &str) -> PathBuf {
        self.root.join(name)  // e.g. PRD.md, TRD.md
    }
    
    pub fn has_artifact(&self, name: &str) -> bool {
        self.artifact_path(name).exists()
    }
    
    pub fn list_artifacts(&self) -> Vec<String> { ... }
}
```

**存储位置**: `.poria/features/<feat-id>/`（项目目录内，非 app 数据目录）。

**与 harness 的映射**: `.poria/features/` 对应 `.harness/features/`，结构兼容：

```
.poria/features/feat-001-batch-export/
├── PRD.md            # 原始需求
├── PRD_REVIEW.md     # 需求澄清（ReviewPrd stage 产出）
├── TRD.md            # 技术设计（Design stage 产出）
├── TASK.md           # 执行计划（Dev stage 产出）
├── CR.md             # 审查报告（CR stage 产出）
├── source/           # 原始资料
└── ui/               # UI 资源
```

### 5. Gate Extensions

在现有 gate 系统基础上增加 dev workflow 特定的 gate 规则：

```rust
// 新增 gate rules
GateRule { name: "prd_review_p0", severity: Block, phase: StageEntry("Design") }
    // 检查: PRD_REVIEW.md 存在且 P0 全部已回答

GateRule { name: "trd_exists", severity: Block, phase: StageEntry("Dev") }
    // 检查: TRD.md 存在

GateRule { name: "code_changes_exist", severity: Block, phase: StageEntry("Cr") }
    // 检查: git diff 非空
```

Gate 评估逻辑从文件系统读取状态（feature context 内的文件），不依赖额外存储。

### 6. Human-in-the-Loop Integration

现有 `HumanLoop` trait 和 `HumanLoopCoordinator` 需要连接到前端：

```
Pipeline reaches human gate
  → PipelineExecutor calls HumanLoop.notify()
  → Tauri event emitted: human:request { pipeline_id, stage, prompt, options }
  → Frontend shows HumanLoopCard
  → User clicks approve/reject/request-changes
  → Frontend calls humanLoopRespond IPC command
  → HumanLoopCoordinator.poll_reply() resolves
  → Pipeline continues or reverts
```

**卡点定义**:
| Stage | 卡点 | 用户操作 |
|-------|------|----------|
| ReviewPrd 后 | 确认 scope + P0 回答 | approve / request-changes |
| Design 后 | 审阅 TRD | approve / request-changes |
| Dev 后 | TASK.md 审阅 | approve（开始编码）/ request-changes |
| Dev 完成后 | 代码审查 | approve / reject |

### 7. Stream Output to Frontend

Claude CLI 流式输出需要实时传递到前端：

```
ClaudeCliSdk → parse stream-json lines
  → emit Tauri event: agent:stream { pipeline_id, stage, chunk_type, content }
  → Frontend EventStream component renders in real-time
```

**事件类型映射**:
- `assistant` text → 显示 Claude 思考/回答
- `tool_use` → 显示工具调用（折叠显示）
- `tool_result` → 显示工具结果（折叠显示）
- `result` → 阶段完成

### 8. AppState Wiring

现有 `AppState` 只有 `store` + `event_store`，需要扩展：

```rust
pub struct AppState {
    pub store: Arc<SqliteStore>,
    pub event_store: Arc<EventStore>,
    // 新增
    pub agent_pool: Arc<ClaudeAgentPool>,
    pub session_tracker: Arc<SessionTracker>,
    pub skill_loader: Arc<SkillLoader>,
    pub human_loop: Arc<HumanLoopCoordinator>,
}
```

`ClaudeAgentPool` 注入 `ClaudeCliSdk` 而非 `MockSdk`。

## Data Flow: Complete Pipeline Run

```
用户提交需求 (PRD link/file)
  │
  ├─ [Platform] JoySpace channel 拉取 PRD → .poria/features/<feat>/PRD.md
  │
  ├─ [AI] ReviewPrd: Claude + prd-review skill
  │   └─ 产出: PRD_REVIEW.md
  │   └─ 人工卡点: 确认 scope, 回答 P0
  │
  ├─ [Gate] prd_review_p0: 检查 P0 已回答
  │
  ├─ [AI] Design: Claude + trd-gen skill
  │   └─ 产出: TRD.md
  │   └─ 人工卡点: 审阅 TRD
  │
  ├─ [Platform] Workspace: 创建分支 + worktree (resource)
  │
  ├─ [Gate] trd_exists: 检查 TRD.md
  │
  ├─ [AI] Dev: Claude + code-impl skill
  │   └─ 产出: TASK.md → 人工审阅 → SubAgent 编码 → OutputGuard
  │
  ├─ [Gate] code_changes_exist: 检查 git diff
  │
  ├─ [AI] Cr: Claude + cr-web skill
  │   └─ 产出: CR.md
  │   └─ 若 score < B+: 回退到 Dev (已有 regress 逻辑)
  │
  └─ [Platform] Deploy: 提 MR + 部署 (channel)
```

## Compatibility

- **不破坏现有架构** — 所有变更都是填充已有的 trait 实现和 stub
- **Fixture 模式保留** — 现有 fixture/mock 行为不受影响，用于测试
- **渐进式实现** — 可以先实现 ClaudeCliSdk，再逐个实现 skill

## Rollback

- 如果 ClaudeCliSdk 有问题，回退到 MockSdk（已有）
- 如果某个 skill 实现有问题，该 skill 回退到 NotImplemented，其他 skill 不受影响
- Feature context 是纯文件系统操作，没有不可逆状态
