// ------------------------------------------------------------
// Module: storage
// Purpose: Write the table to data/state.json shortly after it
//          changes, and read it back on the next boot. Sockets
//          and session tokens are not stored.
// ------------------------------------------------------------

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::map::Scene;
use crate::session::state::{CampaignShelf, PendingStrike, Player, SceneShelf, StoredRoll, Table, Token, UndoStep};
use crate::systems::{Campaign, Character};
use crate::vision::Explored;

const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct SaveFile {
    version: u32,
    players: Vec<Player>,
    scene: Option<Scene>,
    tokens: Vec<Token>,
    explored: Explored,
    rolls: Vec<StoredRoll>,
    #[serde(default)]
    campaign: Campaign,
    #[serde(default)]
    characters: Vec<Character>,
    #[serde(default)]
    scenes: Vec<SceneShelf>,
    #[serde(default)]
    shelved: Vec<CampaignShelf>,
    #[serde(default)]
    fear: u8,
    #[serde(default)]
    strike: Option<PendingStrike>,
    #[serde(default)]
    undo: Option<UndoStep>,
    #[serde(default)]
    talk: Vec<crate::net::protocol::TalkLine>,
}

pub fn load_into(table: &mut Table, data_dir: &Path) {
    let path = data_dir.join("state.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return,
        Err(err) => {
            tracing::warn!("could not read {}: {err}", path.display());
            return;
        }
    };
    match serde_json::from_str::<SaveFile>(&text) {
        Ok(save) if save.version == VERSION => {
            table.players = save
                .players
                .into_iter()
                .map(|mut player| {
                    player.connected = false;
                    player
                })
                .collect();
            table.scene = save.scene;
            table.tokens = save.tokens;
            table.explored = save.explored;
            table.rolls = save.rolls;
            table.campaign = save.campaign;
            table.characters = save.characters;
            table.scenes = save.scenes;
            table.shelved = save.shelved;
            table.fear = save.fear;
            table.strike = save.strike;
            table.undo = save.undo;
            table.talk = save.talk;
            table.ensure_campaign();
            table.sync_fog_size();
            tracing::info!("restored {}", path.display());
        }
        Ok(save) => {
            tracing::warn!("save version {} is not supported; starting fresh", save.version);
            backup(&path);
        }
        Err(err) => {
            tracing::warn!("could not parse {}: {err}", path.display());
            backup(&path);
        }
    }
}

pub fn write(table: &Table, data_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    write_to(table, &data_dir.join("state.json"))
}

pub fn write_to(table: &Table, path: &Path) -> std::io::Result<()> {
    let save = capture(table);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("json.tmp");
    let file = std::fs::File::create(&temp)?;
    serde_json::to_writer_pretty(file, &save).map_err(std::io::Error::other)?;
    std::fs::rename(&temp, path)?;
    Ok(())
}

pub fn write_fresh(path: &Path, name: &str) -> std::io::Result<()> {
    let mut table = Table::new();
    table.campaign.name = name.to_string();
    write_to(&table, path)
}

pub fn load_file(table: &mut Table, path: &Path) -> bool {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => {
            tracing::warn!("could not read {}: {err}", path.display());
            return false;
        }
    };
    match serde_json::from_str::<SaveFile>(&text) {
        Ok(save) if save.version == VERSION => {
            apply(table, save);
            true
        }
        Ok(save) => {
            tracing::warn!("save version {} is not supported", save.version);
            false
        }
        Err(err) => {
            tracing::warn!("could not parse {}: {err}", path.display());
            false
        }
    }
}

fn capture(table: &Table) -> SaveFile {
    SaveFile {
        version: VERSION,
        players: table.players.clone(),
        scene: table.scene.clone(),
        tokens: table.tokens.clone(),
        explored: table.explored.clone(),
        rolls: table.rolls.clone(),
        campaign: table.campaign.clone(),
        characters: table.characters.clone(),
        scenes: table.scenes.clone(),
        shelved: table.shelved.clone(),
        fear: table.fear,
        strike: table.strike.clone(),
        undo: table.undo.clone(),
        talk: table.talk.clone(),
    }
}

fn apply(table: &mut Table, save: SaveFile) {
    table.players = save
        .players
        .into_iter()
        .map(|mut player| {
            player.connected = false;
            player
        })
        .collect();
    table.scene = save.scene;
    table.tokens = save.tokens;
    table.explored = save.explored;
    table.rolls = save.rolls;
    table.campaign = save.campaign;
    table.characters = save.characters;
    table.scenes = save.scenes;
    table.shelved = save.shelved;
    table.fear = save.fear;
    table.strike = save.strike;
    table.undo = save.undo;
    table.talk = save.talk;
    table.ensure_campaign();
    table.sync_fog_size();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_roll_without_duality_still_loads() {
        let old = r#"{
            "id":"r","roller_id":"a","roller_name":"Ada","formula":"1d20","label":null,
            "visibility":"public","modifier":0,"total":5,
            "dice":[{"sides":20,"value":5,"sign":1}],"created_at":1
        }"#;
        let roll: StoredRoll = serde_json::from_str(old).unwrap();
        assert_eq!(roll.dice[0].tag, None);
        assert!(roll.outcome.is_none());
    }

    #[test]
    fn an_old_table_without_characters_still_loads() {
        let dir = std::env::temp_dir().join(format!("game-table-old-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let old = r##"{
            "version":1,"players":[],"scene":null,
            "tokens":[{"id":"t","name":"Ada","x":1.5,"y":1.5,"image_file":null,"color":"#fff","owner_id":null,"hidden":false,"vision_radius":4,"grants_vision":true}],
            "explored":{"width":1,"height":1,"bits":"AA=="},
            "rolls":[]
        }"##;
        std::fs::write(dir.join("state.json"), old).unwrap();
        let mut table = Table::new();
        load_into(&mut table, &dir);
        assert!(table.campaign.system_id.is_none());
        assert_eq!(table.campaign.name, "Campaign");
        assert!(!table.campaign.id.is_empty());
        assert!(table.scenes.is_empty());
        assert!(table.shelved.is_empty());
        assert!(table.characters.is_empty());
        assert_eq!(table.tokens[0].character_id, None);
        assert_eq!(table.fear, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_migration_fixture_loads_without_touching_the_user_save() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v1-mixed.json");
        assert!(source.ends_with("tests/fixtures/v1-mixed.json"));
        let user = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/state.json");
        assert_ne!(source, user);
        let fixture = include_str!("../tests/fixtures/v1-mixed.json");
        let dir = std::env::temp_dir().join(format!("longtable-fixture-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        std::fs::write(&path, fixture).unwrap();
        let mut table = Table::new();
        assert!(load_file(&mut table, &path));
        assert_eq!(table.scene.as_ref().unwrap().name, "Hall");
        assert_eq!(table.scene.as_ref().unwrap().image_file.as_deref(), Some("missing-map.png"));
        assert!(!table.scene.as_ref().unwrap().doors[0].open);
        assert_eq!(table.tokens.len(), 2);
        assert_eq!(table.tokens[0].character_id.as_deref(), Some("ada"));
        assert!(table.tokens[1].character_id.is_none());
        assert_eq!(table.tokens[1].image_file.as_deref(), Some("missing-portrait.png"));
        assert_eq!(table.scenes.len(), 1);
        assert_eq!(table.scenes[0].name, "Cellar");
        assert_eq!(table.shelved.len(), 1);
        assert_eq!(table.characters.len(), 1);
        assert_eq!(table.shelved[0].characters.len(), 1);
        assert_eq!(table.fear, 3);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!user.ends_with("tests/fixtures/v1-mixed.json"));
    }
}

fn backup(path: &Path) {
    let mut bak = PathBuf::from(path);
    bak.set_extension("json.bak");
    if let Err(err) = std::fs::rename(path, &bak) {
        tracing::warn!("could not move the unreadable save aside: {err}");
    }
}
