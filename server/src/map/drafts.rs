// ------------------------------------------------------------
// Module: drafts
// Purpose: One JSON file per map the GM is building. Publishing
//          is separate, so players never see a half-drawn map.
// ------------------------------------------------------------

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::map::layout::MapLayout;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub id: String,
    pub name: String,
    pub updated_at: u64,
    pub layout: MapLayout,
    /// Scene this draft last published. The next publish updates that scene.
    #[serde(default)]
    pub scene_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftSummary {
    pub id: String,
    pub name: String,
    pub updated_at: u64,
    pub width: u32,
    pub height: u32,
}

pub struct DraftStore {
    dir: PathBuf,
}

impl DraftStore {
    pub fn new(data_dir: &Path) -> Self {
        Self { dir: data_dir.join("maps") }
    }

    pub fn list(&self) -> Vec<DraftSummary> {
        let mut summaries = Vec::new();
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return summaries;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            if let Ok(draft) = read_draft(&path) {
                summaries.push(DraftSummary {
                    id: draft.id,
                    name: draft.name,
                    updated_at: draft.updated_at,
                    width: draft.layout.width,
                    height: draft.layout.height,
                });
            }
        }
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        summaries
    }

    pub fn get(&self, id: &str) -> Result<Draft, String> {
        let id = safe_id(id)?;
        read_draft(&self.path(&id))
    }

    pub fn put(&self, draft: &Draft) -> Result<(), String> {
        safe_id(&draft.id)?;
        draft.layout.validate()?;
        if draft.name.trim().is_empty() || draft.name.chars().count() > 48 {
            return Err("give the map a name of at most 48 characters".into());
        }
        std::fs::create_dir_all(&self.dir).map_err(|err| err.to_string())?;
        let path = self.path(&draft.id);
        let temp = path.with_extension("json.tmp");
        let file = std::fs::File::create(&temp).map_err(|err| err.to_string())?;
        serde_json::to_writer_pretty(file, draft).map_err(|err| err.to_string())?;
        std::fs::rename(&temp, &path).map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let id = safe_id(id)?;
        let path = self.path(&id);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Err("that draft is not saved".into()),
            Err(err) => Err(err.to_string()),
        }
    }

    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }
}

fn read_draft(path: &Path) -> Result<Draft, String> {
    let text = std::fs::read_to_string(path).map_err(|_| "that draft is not saved".to_string())?;
    serde_json::from_str(&text).map_err(|err| format!("could not read the draft: {err}"))
}

pub fn safe_id(id: &str) -> Result<String, String> {
    if (8..=64).contains(&id.len()) && id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
        Ok(id.to_string())
    } else {
        Err("that draft id is not usable".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_draft_round_trips_through_a_file() {
        let dir = std::env::temp_dir().join(format!("game-table-drafts-{}", uuid::Uuid::new_v4()));
        let store = DraftStore { dir: dir.clone() };
        let draft = Draft {
            id: "draft-0001".into(),
            name: "Glade".into(),
            updated_at: 5,
            layout: MapLayout::blank(8, 6, "core/grass"),
            scene_id: None,
        };
        store.put(&draft).unwrap();
        let loaded = store.get("draft-0001").unwrap();
        assert_eq!(loaded.name, "Glade");
        assert_eq!(loaded.layout.width, 8);
        assert_eq!(loaded.layout.cells, draft.layout.cells);
        store.delete("draft-0001").unwrap();
        assert!(store.get("draft-0001").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
