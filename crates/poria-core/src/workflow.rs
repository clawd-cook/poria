use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::types::{StageEnum, StageStatus};

pub const DEFAULT_WORKFLOW_ID: &str = "demand-to-mr";

pub const BUNDLED_DEMAND_TO_MR_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../workflows/demand-to-mr.yml"
));

pub const REGISTERED_ACTIONS: &[&str] = &[
    "skill:init",
    "skill:review-prd",
    "skill:gen-trd",
    "skill:test-plan",
    "skill:gen-code",
    "skill:lint",
    "skill:code-review",
    "skill:test-cases",
    "skill:run-autotest",
    "skill:handoff-qa",
    "skill:deploy",
    "skill:archive",
    "poria/dev-verify",
    "poria/post-cr-notes",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowDocument {
    pub name: String,
    pub jobs: Vec<WorkflowJob>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowJob {
    pub id: String,
    pub needs: Vec<String>,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub uses: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedJob {
    pub id: StageEnum,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<String>,
    pub steps: Vec<MaterializedStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedStep {
    pub id: String,
    pub uses: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkflowError {
    #[error("{0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepRunStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRecord {
    pub id: String,
    pub uses: String,
    pub status: StepRunStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl WorkflowDocument {
    pub fn job(&self, id: &str) -> Option<&WorkflowJob> {
        self.jobs.iter().find(|job| job.id == id)
    }
}

pub fn parse_workflow_yaml(yaml: &str) -> Result<WorkflowDocument, WorkflowError> {
    let value: serde_yaml::Value =
        serde_yaml::from_str(yaml).map_err(|err| WorkflowError::Invalid(err.to_string()))?;
    parse_workflow_value(&value)
}

pub fn parse_and_validate(yaml: &str) -> Result<WorkflowDocument, WorkflowError> {
    let doc = parse_workflow_yaml(yaml)?;
    validate_workflow(&doc)?;
    Ok(doc)
}

pub fn bundled_demand_to_mr() -> WorkflowDocument {
    parse_and_validate(BUNDLED_DEMAND_TO_MR_YAML)
        .expect("bundled workflows/demand-to-mr.yml must parse")
}

pub fn fallback_workflow(config_jobs: &[MaterializedJob]) -> WorkflowDocument {
    if config_jobs.is_empty() {
        bundled_demand_to_mr()
    } else {
        document_from_materialized(DEFAULT_WORKFLOW_ID, config_jobs)
    }
}

pub fn document_from_materialized(name: &str, jobs: &[MaterializedJob]) -> WorkflowDocument {
    WorkflowDocument {
        name: name.to_string(),
        jobs: jobs
            .iter()
            .map(|job| WorkflowJob {
                id: job.id.as_str().to_string(),
                needs: job.needs.clone(),
                steps: job
                    .steps
                    .iter()
                    .map(|step| WorkflowStep {
                        uses: step.uses.clone(),
                        id: Some(step.id.clone()),
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub fn materialize(doc: &WorkflowDocument) -> Result<Vec<MaterializedJob>, WorkflowError> {
    validate_workflow(doc)?;
    doc.jobs
        .iter()
        .map(|job| {
            let id = StageEnum::from_job_id(&job.id)
                .ok_or_else(|| WorkflowError::Invalid(format!("未知 Job id: {}", job.id)))?;
            Ok(MaterializedJob {
                id,
                needs: job.needs.clone(),
                steps: job
                    .steps
                    .iter()
                    .enumerate()
                    .map(|(index, step)| MaterializedStep {
                        id: step_id(step, index),
                        uses: step.uses.clone(),
                    })
                    .collect(),
            })
        })
        .collect()
}

pub fn validate_workflow(doc: &WorkflowDocument) -> Result<(), WorkflowError> {
    if doc.name.trim().is_empty() {
        return Err(WorkflowError::Invalid("workflow name 不能为空".into()));
    }
    if doc.jobs.is_empty() {
        return Err(WorkflowError::Invalid("workflow 至少需要一个 Job".into()));
    }

    let mut seen = HashSet::new();
    for job in &doc.jobs {
        if job.id.trim().is_empty() {
            return Err(WorkflowError::Invalid("Job id 不能为空".into()));
        }
        if StageEnum::from_job_id(&job.id).is_none() {
            return Err(WorkflowError::Invalid(format!(
                "未知 Job id: {}（允许 STAGE_ORDER 全链路 id 及旧六阶段别名）",
                job.id
            )));
        }
        if !seen.insert(job.id.as_str()) {
            return Err(WorkflowError::Invalid(format!("重复 Job id: {}", job.id)));
        }
        if job.steps.is_empty() {
            return Err(WorkflowError::Invalid(format!(
                "Job {} 至少需要一个 step",
                job.id
            )));
        }
        for step in &job.steps {
            if !REGISTERED_ACTIONS.contains(&step.uses.as_str()) {
                return Err(WorkflowError::Invalid(format!(
                    "非法 uses: {}（必须落在 Action 注册表）",
                    step.uses
                )));
            }
        }
    }

    let ids: HashSet<&str> = doc.jobs.iter().map(|job| job.id.as_str()).collect();
    for job in &doc.jobs {
        for need in &job.needs {
            if !ids.contains(need.as_str()) {
                return Err(WorkflowError::Invalid(format!(
                    "Job {} 的 needs 引用了未知 Job: {}",
                    job.id, need
                )));
            }
            if need == &job.id {
                return Err(WorkflowError::Invalid(format!(
                    "Job {} 不能依赖自己",
                    job.id
                )));
            }
        }
    }

    detect_cycle(doc)?;
    Ok(())
}

/// Jobs whose `needs` are all completed and that are not themselves completed.
/// Document order is preserved so v1 can pick the first ready job.
pub fn ready_jobs<'a>(doc: &'a WorkflowDocument, completed: &HashSet<&str>) -> Vec<&'a str> {
    doc.jobs
        .iter()
        .filter(|job| !completed.contains(job.id.as_str()))
        .filter(|job| {
            job.needs
                .iter()
                .all(|need| completed.contains(need.as_str()))
        })
        .map(|job| job.id.as_str())
        .collect()
}

pub fn completed_job_ids(stages: &[crate::types::Stage]) -> HashSet<&str> {
    stages
        .iter()
        .filter(|stage| matches!(stage.status, StageStatus::Completed | StageStatus::Skipped))
        .map(|stage| stage.name.as_str())
        .collect()
}

pub fn next_ready_stage_index(
    stages: &[crate::types::Stage],
    doc: &WorkflowDocument,
) -> Option<usize> {
    let completed = completed_job_ids(stages);
    let ready = ready_jobs(doc, &completed);
    for job_id in ready {
        if let Some(idx) = stages
            .iter()
            .position(|stage| stage.name.as_str() == job_id)
        {
            if !matches!(
                stages[idx].status,
                StageStatus::Completed | StageStatus::Skipped
            ) {
                return Some(idx);
            }
        }
    }
    None
}

pub fn steps_for_job(jobs: &[MaterializedJob], name: StageEnum) -> Option<&[MaterializedStep]> {
    jobs.iter()
        .find(|job| job.id == name)
        .map(|job| job.steps.as_slice())
}

pub fn default_steps_for_stage(name: StageEnum) -> Vec<MaterializedStep> {
    bundled_demand_to_mr()
        .job(name.as_str())
        .map(|job| {
            job.steps
                .iter()
                .enumerate()
                .map(|(index, step)| MaterializedStep {
                    id: step_id(step, index),
                    uses: step.uses.clone(),
                })
                .collect()
        })
        .unwrap_or_else(|| {
            let uses = default_skill_uses(name);
            vec![MaterializedStep {
                id: uses.to_string(),
                uses: uses.to_string(),
            }]
        })
}

pub fn default_skill_uses(name: StageEnum) -> &'static str {
    match name {
        StageEnum::Init => "skill:init",
        StageEnum::ReviewPrd => "skill:review-prd",
        StageEnum::Design => "skill:gen-trd",
        StageEnum::TestPlan => "skill:test-plan",
        StageEnum::Dev => "skill:gen-code",
        StageEnum::Lint => "skill:lint",
        StageEnum::Cr => "skill:code-review",
        StageEnum::TestCases => "skill:test-cases",
        StageEnum::RunAutotest => "skill:run-autotest",
        StageEnum::HandoffQa => "skill:handoff-qa",
        StageEnum::Deploy => "skill:deploy",
        StageEnum::Archive => "skill:archive",
    }
}

pub fn is_registered_action(uses: &str) -> bool {
    REGISTERED_ACTIONS.contains(&uses)
}

fn step_id(step: &WorkflowStep, index: usize) -> String {
    step.id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("{}@{index}", step.uses))
}

fn parse_workflow_value(value: &serde_yaml::Value) -> Result<WorkflowDocument, WorkflowError> {
    let mapping = value
        .as_mapping()
        .ok_or_else(|| WorkflowError::Invalid("workflow 必须是 YAML mapping".into()))?;
    let name = mapping
        .get(serde_yaml::Value::from("name"))
        .and_then(serde_yaml::Value::as_str)
        .unwrap_or(DEFAULT_WORKFLOW_ID)
        .trim()
        .to_string();
    let jobs_value = mapping
        .get(serde_yaml::Value::from("jobs"))
        .ok_or_else(|| WorkflowError::Invalid("缺少 jobs".into()))?;
    let jobs_map = jobs_value
        .as_mapping()
        .ok_or_else(|| WorkflowError::Invalid("jobs 必须是 mapping".into()))?;

    let mut jobs = Vec::new();
    for (key, job_value) in jobs_map {
        let id = key
            .as_str()
            .ok_or_else(|| WorkflowError::Invalid("Job id 必须是字符串".into()))?
            .to_string();
        jobs.push(parse_job(&id, job_value)?);
    }
    Ok(WorkflowDocument { name, jobs })
}

fn parse_job(id: &str, value: &serde_yaml::Value) -> Result<WorkflowJob, WorkflowError> {
    let mapping = value
        .as_mapping()
        .ok_or_else(|| WorkflowError::Invalid(format!("Job {id} 必须是 mapping")))?;
    let needs = match mapping.get(serde_yaml::Value::from("needs")) {
        None => Vec::new(),
        Some(serde_yaml::Value::String(need)) => vec![need.clone()],
        Some(serde_yaml::Value::Sequence(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(ToOwned::to_owned)
                    .ok_or_else(|| WorkflowError::Invalid(format!("Job {id} needs 必须是字符串")))
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => {
            return Err(WorkflowError::Invalid(format!(
                "Job {id} needs 必须是字符串或字符串列表"
            )))
        }
    };
    let steps_value = mapping
        .get(serde_yaml::Value::from("steps"))
        .ok_or_else(|| WorkflowError::Invalid(format!("Job {id} 缺少 steps")))?;
    let steps_seq = steps_value
        .as_sequence()
        .ok_or_else(|| WorkflowError::Invalid(format!("Job {id} steps 必须是列表")))?;
    let mut steps = Vec::new();
    for (index, step_value) in steps_seq.iter().enumerate() {
        steps.push(parse_step(id, index, step_value)?);
    }
    Ok(WorkflowJob {
        id: id.to_string(),
        needs,
        steps,
    })
}

fn parse_step(
    job_id: &str,
    index: usize,
    value: &serde_yaml::Value,
) -> Result<WorkflowStep, WorkflowError> {
    let mapping = value.as_mapping().ok_or_else(|| {
        WorkflowError::Invalid(format!("Job {job_id} step {index} 必须是 mapping"))
    })?;
    if mapping.contains_key(serde_yaml::Value::from("run")) {
        return Err(WorkflowError::Invalid(
            "不允许 run: 任意 shell，uses 必须落在 Action 注册表".into(),
        ));
    }
    let uses = mapping
        .get(serde_yaml::Value::from("uses"))
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|uses| !uses.is_empty())
        .ok_or_else(|| WorkflowError::Invalid(format!("Job {job_id} step {index} 缺少 uses")))?;
    let id = mapping
        .get(serde_yaml::Value::from("id"))
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(ToOwned::to_owned);
    Ok(WorkflowStep {
        uses: uses.to_string(),
        id,
    })
}

fn detect_cycle(doc: &WorkflowDocument) -> Result<(), WorkflowError> {
    let index: HashMap<&str, usize> = doc
        .jobs
        .iter()
        .enumerate()
        .map(|(i, job)| (job.id.as_str(), i))
        .collect();
    let mut visiting = vec![0u8; doc.jobs.len()];
    for i in 0..doc.jobs.len() {
        visit(doc, i, &index, &mut visiting)?;
    }
    Ok(())
}

fn visit(
    doc: &WorkflowDocument,
    i: usize,
    index: &HashMap<&str, usize>,
    visiting: &mut [u8],
) -> Result<(), WorkflowError> {
    match visiting[i] {
        1 => {
            return Err(WorkflowError::Invalid(format!(
                "Job needs 存在环: {}",
                doc.jobs[i].id
            )))
        }
        2 => return Ok(()),
        _ => visiting[i] = 1,
    }
    for need in &doc.jobs[i].needs {
        if let Some(&j) = index.get(need.as_str()) {
            visit(doc, j, index, visiting)?;
        }
    }
    visiting[i] = 2;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PipelineConfig, Stage, STAGE_ORDER};

    fn yaml_with_parallel_after_init() -> &'static str {
        r#"
name: parallel-after-init
jobs:
  init:
    steps:
      - uses: skill:init
  review_prd:
    needs: [init]
    steps:
      - uses: skill:review-prd
  design:
    needs: [init]
    steps:
      - uses: skill:gen-trd
"#
    }

    #[test]
    fn bundled_demand_to_mr_is_linear_stage_order() {
        let doc = bundled_demand_to_mr();
        assert_eq!(doc.name, "demand-to-mr");
        let ids: Vec<_> = doc.jobs.iter().map(|job| job.id.as_str()).collect();
        assert_eq!(
            ids,
            STAGE_ORDER
                .iter()
                .map(|stage| stage.as_str())
                .collect::<Vec<_>>()
        );
        let mut completed = HashSet::new();
        for stage in STAGE_ORDER {
            let ready = ready_jobs(&doc, &completed);
            assert_eq!(ready, vec![stage.as_str()]);
            completed.insert(stage.as_str());
        }
        assert!(ready_jobs(&doc, &completed).is_empty());
    }

    #[test]
    fn bundled_dev_and_deploy_include_builtin_steps() {
        let doc = bundled_demand_to_mr();
        let dev_uses: Vec<_> = doc
            .job("implement")
            .unwrap()
            .steps
            .iter()
            .map(|step| step.uses.as_str())
            .collect();
        assert_eq!(dev_uses, vec!["skill:gen-code", "poria/dev-verify"]);
        let deploy_uses: Vec<_> = doc
            .job("deploy")
            .unwrap()
            .steps
            .iter()
            .map(|step| step.uses.as_str())
            .collect();
        assert_eq!(deploy_uses, vec!["skill:deploy", "poria/post-cr-notes"]);
    }

    #[test]
    fn two_jobs_needing_same_upstream_are_both_ready() {
        let doc = parse_and_validate(yaml_with_parallel_after_init()).unwrap();
        let mut completed = HashSet::new();
        completed.insert("init");
        let ready = ready_jobs(&doc, &completed);
        assert_eq!(ready, vec!["review_prd", "design"]);
    }

    #[test]
    fn unknown_job_id_and_uses_are_rejected() {
        let unknown_job = parse_workflow_yaml(
            r#"
name: bad
jobs:
  workspace:
    steps:
      - uses: skill:init
"#,
        )
        .unwrap();
        assert!(validate_workflow(&unknown_job).is_err());

        let unknown_uses = parse_workflow_yaml(
            r#"
name: bad
jobs:
  init:
    steps:
      - uses: skill:not-a-thing
"#,
        )
        .unwrap();
        assert!(validate_workflow(&unknown_uses).is_err());
    }

    #[test]
    fn arbitrary_run_shell_is_rejected() {
        let err = parse_workflow_yaml(
            r#"
name: bad
jobs:
  init:
    steps:
      - run: echo hi
"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("不允许 run"));
    }

    #[test]
    fn cycle_is_rejected() {
        let doc = parse_workflow_yaml(
            r#"
name: cycle
jobs:
  init:
    needs: [design]
    steps:
      - uses: skill:init
  design:
    needs: [init]
    steps:
      - uses: skill:gen-trd
"#,
        )
        .unwrap();
        assert!(validate_workflow(&doc).is_err());
    }

    #[test]
    fn materialize_round_trips_job_ids() {
        let jobs = materialize(&bundled_demand_to_mr()).unwrap();
        assert_eq!(jobs.len(), STAGE_ORDER.len());
        assert_eq!(jobs[0].id, StageEnum::Init);
        let implement = jobs
            .iter()
            .find(|job| job.id == StageEnum::Dev)
            .expect("implement job");
        assert_eq!(implement.steps[1].uses, "poria/dev-verify");
        let deploy = jobs
            .iter()
            .find(|job| job.id == StageEnum::Deploy)
            .expect("deploy job");
        assert_eq!(deploy.steps[1].uses, "poria/post-cr-notes");
        let restored = document_from_materialized("demand-to-mr", &jobs);
        let deploy_restored = restored
            .jobs
            .iter()
            .find(|job| job.id == "deploy")
            .expect("deploy");
        assert_eq!(deploy_restored.steps[1].uses, "poria/post-cr-notes");
    }

    #[test]
    fn next_ready_stage_index_skips_completed_and_blocked_upstream() {
        let doc = bundled_demand_to_mr();
        let stages = STAGE_ORDER
            .iter()
            .map(|name| Stage {
                id: None,
                pipeline_id: "p1".into(),
                name: *name,
                status: if *name == StageEnum::Init {
                    StageStatus::Completed
                } else {
                    StageStatus::Pending
                },
                skill_id: None,
                retry_count: 0,
                max_retries: 3,
                input: None,
                output: None,
                gate_results: None,
                issue: None,
                rollback: None,
                agent_session_id: None,
                started_at: None,
                completed_at: None,
            })
            .collect::<Vec<_>>();
        let idx = next_ready_stage_index(&stages, &doc).unwrap();
        assert_eq!(stages[idx].name, StageEnum::ReviewPrd);
    }

    #[test]
    fn pipeline_config_deserializes_without_workflow_fields() {
        let config: PipelineConfig =
            serde_json::from_str(r#"{"gates":[],"trd_scope":[],"repos":[]}"#).unwrap();
        assert!(config.workflow_id.is_none());
        assert!(config.jobs.is_empty());
    }
}
