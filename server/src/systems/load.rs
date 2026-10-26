// ------------------------------------------------------------
// Module: load
// Purpose: Read each system's folder and write one back.
//          A system that does not parse is logged and skipped,
//          so one broken file does not stop the table.
// ------------------------------------------------------------

use std::path::Path;

use super::def::{adopt_legacy, validate, SystemDef};

#[derive(Default)]
pub struct SystemCatalog {
    pub list: Vec<SystemDef>,
}

impl SystemCatalog {
    pub fn get(&self, id: &str) -> Option<&SystemDef> {
        self.list.iter().find(|system| system.id == id)
    }

    pub fn upsert(&mut self, system: SystemDef) {
        if let Some(slot) = self.list.iter_mut().find(|item| item.id == system.id) {
            *slot = system;
        } else {
            self.list.push(system);
        }
        self.list.sort_by(|left, right| left.name.cmp(&right.name));
    }
}

pub fn load_systems(dir: &Path) -> SystemCatalog {
    let mut catalog = SystemCatalog::default();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return catalog,
    };
    for entry in entries.flatten() {
        let path = entry.path().join("system.json");
        if !path.is_file() {
            continue;
        }
        let folder = entry.file_name().to_string_lossy().to_string();
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                tracing::warn!("could not read {}: {err}", path.display());
                continue;
            }
        };
        let mut system: SystemDef = match serde_json::from_str(&text) {
            Ok(system) => system,
            Err(err) => {
                tracing::warn!("could not parse {}: {err}", path.display());
                continue;
            }
        };
        adopt_legacy(&mut system);
        if system.id != folder {
            tracing::warn!("{} says its id is {}, so it was skipped", path.display(), system.id);
            continue;
        }
        if let Err(err) = validate(&system) {
            tracing::warn!("{}: {err}", path.display());
            continue;
        }
        catalog.list.push(system);
    }
    catalog.list.sort_by(|left, right| left.name.cmp(&right.name));
    catalog
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_systems_load_and_daggerheart_keeps_its_pair() {
        let catalog = load_systems(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../systems").as_path());
        assert!(catalog.get("daggerheart").is_some());
        assert!(catalog.get("pathfinder2e").is_some());
        assert!(catalog.get("shadowdark").is_some());
        assert!(catalog.get("nimble").is_some());
        let heart = catalog.get("daggerheart").unwrap();
        assert!(heart.die_tags.iter().any(|tag| tag.letter == "h"));
        assert!(heart.actions.iter().any(|action| action.formula.contains("@agility")));
    }
}

pub fn save_system(dir: &Path, system: &SystemDef) -> Result<SystemDef, String> {
    let mut system = system.clone();
    adopt_legacy(&mut system);
    validate(&system)?;
    let folder = dir.join(&system.id);
    std::fs::create_dir_all(&folder).map_err(|err| err.to_string())?;
    let path = folder.join("system.json");
    let temp = folder.join("system.json.tmp");
    let file = std::fs::File::create(&temp).map_err(|err| err.to_string())?;
    serde_json::to_writer_pretty(file, &system).map_err(|err| err.to_string())?;
    std::fs::rename(&temp, &path).map_err(|err| err.to_string())?;
    Ok(system)
}

#[cfg(test)]
mod legacy_tests {
    use super::*;
    use crate::systems::FieldKind;

    #[test]
    fn a_legacy_sheet_is_copied_onto_every_kind() {
        let mut system: SystemDef = serde_json::from_str(
            r#"{
                "id": "oldgame",
                "name": "Old",
                "fields": [{"id": "hp", "name": "HP", "kind": "track", "max": 6}],
                "sheet": {"sections": [{"title": "Vitals", "columns": 1, "fields": ["hp"]}]},
                "toolbar": {"resources": ["hp"], "actions": []}
            }"#,
        )
        .unwrap();
        adopt_legacy(&mut system);
        assert_eq!(system.fields[0].kind, FieldKind::Pool);
        assert_eq!(system.sheets.pc.sections[0].title, "Vitals");
        assert_eq!(system.sheets.npc.sections[0].fields, vec!["hp".to_string()]);
        assert_eq!(system.sheets.adversary.toolbar.resources, vec!["hp".to_string()]);
        validate(&system).unwrap();
        let written = serde_json::to_value(&system).unwrap();
        assert!(written.get("sheet").is_none());
        assert!(written.get("sheets").is_some());
    }
}
