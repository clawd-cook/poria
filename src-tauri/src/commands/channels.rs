use poria_core::contracts::Channel;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct ChannelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
}

/// Return metadata for all registered channels.
#[tauri::command]
pub async fn list_channels() -> Result<Vec<ChannelInfo>, String> {
    let channels: Vec<Box<dyn Channel>> = vec![
        Box::new(poria_channels::xingyun::create_xingyun_channel()),
        Box::new(poria_channels::coding::create_coding_channel()),
        Box::new(poria_channels::jme::create_jme_channel()),
        Box::new(poria_channels::joyspace::create_joyspace_channel()),
        Box::new(poria_channels::defect::create_defect_channel()),
    ];

    Ok(channels
        .iter()
        .map(|c| {
            let meta = c.metadata();
            ChannelInfo {
                id: meta.id.clone(),
                name: meta.name.clone(),
                description: meta.description.clone(),
                version: meta.version.clone(),
            }
        })
        .collect())
}
