// ------------------------------------------------------------
// Module: drafts
// Purpose: One JSON file per map the GM is building. Publishing
//          is separate, so players never see a half-drawn map.
// ------------------------------------------------------------

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::map::layout::{MapLayout, EnvironmentLighting};

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
    /// Ambient settings when this draft was copied or last published.
    /// Comparing these with layout.environment preserves unpublished edits after reload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_base: Option<EnvironmentLighting>,
}

impl Draft {
    /// Follow newer live lighting only when this draft has not changed its ambient settings.
    pub fn follow_live_environment(&mut self, live: &EnvironmentLighting) {
        if self.layout.environment.clone().unwrap_or_default() == self.environment_base.clone().unwrap_or_default() {
            self.layout.environment = Some(live.clone());
        }
    }

    pub fn mark_environment_published(&mut self) {
        self.environment_base = Some(self.layout.environment.clone().unwrap_or_default());
    }
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
        if let Some(base) = &draft.environment_base { base.validate()?; }
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

/// A save may replace a draft only when it still has the revision the GM loaded.
pub fn check_revision(stored: Option<u64>, sent: u64) -> Result<(), String> {
    if stored.is_some_and(|stored| stored != sent) {
        return Err("this map was saved somewhere else. Reload it, then save again.".into());
    }
    Ok(())
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
    fn ambient_edit_base_survives_reload_and_advances_after_publish() {
        use crate::map::layout::EnvironmentPreset;
        let dir = std::env::temp_dir().join(format!("game-table-draft-light-{}", uuid::Uuid::new_v4()));
        let store = DraftStore { dir: dir.clone() };
        let day = EnvironmentLighting::default();
        let night = EnvironmentLighting { preset: EnvironmentPreset::Night, brightness: 0.16 };
        let dusk = EnvironmentLighting { preset: EnvironmentPreset::Dusk, brightness: 0.55 };
        let mut draft = Draft {
            id: "draft-0002".into(), name: "Lighting".into(), updated_at: 1,
            layout: MapLayout::blank(8, 8, "core/grass"), scene_id: Some("live-scene".into()),
            environment_base: Some(day.clone()),
        };
        draft.layout.environment = Some(night.clone());
        store.put(&draft).unwrap();
        let mut reloaded = store.get(&draft.id).unwrap();
        reloaded.follow_live_environment(&dusk);
        assert_eq!(reloaded.layout.environment.as_ref(), Some(&night), "an unpublished intentional change survives reload");
        reloaded.mark_environment_published();
        store.put(&reloaded).unwrap();
        let mut published = store.get(&draft.id).unwrap();
        assert_eq!(published.environment_base.as_ref(), Some(&night));
        published.follow_live_environment(&dusk);
        assert_eq!(published.layout.environment.as_ref(), Some(&dusk), "an unchanged published draft follows live lighting");
        published.mark_environment_published();
        assert_eq!(published.environment_base.as_ref(), Some(&dusk));
        let mut legacy = serde_json::to_value(&draft).unwrap();
        legacy.as_object_mut().unwrap().remove("environmentBase");
        assert!(serde_json::from_value::<Draft>(legacy).unwrap().environment_base.is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

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
            environment_base: Some(EnvironmentLighting::default()),
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

    #[test]
    fn an_older_save_cannot_replace_a_newer_one() {
        assert!(check_revision(None, 0).is_ok());
        assert!(check_revision(Some(5), 5).is_ok());
        let err = check_revision(Some(9), 5).unwrap_err();
        assert!(err.contains("saved somewhere else"), "{err}");
    }
}
