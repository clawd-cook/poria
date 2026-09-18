use poria_core::contracts::Skill;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
}

/// Return metadata for all registered skills.
#[tauri::command]
pub async fn list_skills() -> Result<Vec<SkillInfo>, String> {
    let skills: Vec<Box<dyn Skill>> = vec![
        Box::new(poria_skills::InitSkill::new()),
        Box::new(poria_skills::ReviewPrdSkill::new()),
        Box::new(poria_skills::GenTrdSkill::new()),
        Box::new(poria_skills::GenCodeSkill::new()),
        Box::new(poria_skills::CodeReviewSkill::new()),
        Box::new(poria_skills::DeploySkill::new()),
    ];

    Ok(skills
        .iter()
        .map(|s| {
            let meta = s.metadata();
            SkillInfo {
                id: meta.id.clone(),
                name: meta.name.clone(),
                description: meta.description.clone(),
                version: meta.version.clone(),
            }
        })
        .collect())
}
