// ------------------------------------------------------------
// Module: storage
// Purpose: Write the table to data/state.json shortly after it
//          changes, and read it back on the next boot. Sockets
//          and session tokens are not stored.
// ------------------------------------------------------------

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::map::Scene;
use crate::session::state::{CampaignShelf, PendingStrike, Player, SceneFog, SceneShelf, StoredRoll, Table, Token, UndoStep};
use crate::systems::{Campaign, Character};
use crate::vision::Explored;

const VERSION: u32 = 2;

#[derive(Serialize, Deserialize)]
struct SaveFile {
    version: u32,
    players: Vec<Player>,
    scene: Option<Scene>,
    tokens: Vec<Token>,
    explored: Explored,
    #[serde(default)]
    fog_memory: HashMap<String, SceneFog>,
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
    if let Err(err) = load_file(table, &path) {
        if err.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!("{err}");
        }
    }
}

pub fn write(table: &Table, data_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    write_to(table, &data_dir.join("state.json"))
}

pub fn write_to(table: &Table, path: &Path) -> std::io::Result<()> {
    if let Some(hold) = &table.save_hold {
        return Err(std::io::Error::other(format!("the save is on hold: {hold}")));
    }
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

pub fn load_file(table: &mut Table, path: &Path) -> std::io::Result<bool> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err),
    };
    match open_save(&text) {
        Open::Ready(opened) => {
            if opened.save.version < VERSION {
                let _ = keep_backup(path);
            }
            let older = opened.save.version < VERSION;
            let dropped_undo = opened.dropped_undo;
            apply(table, opened.save);
            table.save_hold = None;
            table.review = if older { Some(migration_notes(table, dropped_undo)) } else { None };
            tracing::info!("restored {}", path.display());
            Ok(true)
        }
        Open::Hold(reason) => {
            let copy = keep_backup(path).map(|path| path.display().to_string()).unwrap_or_else(|err| err.to_string());
            let message = format!("{reason} The original file was left in place. A copy is {copy}.");
            table.save_hold = Some(message.clone());
            tracing::warn!("{message}");
            Err(std::io::Error::other(message))
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
        fog_memory: table.fog_memory.clone(),
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

struct Opened {
    save: SaveFile,
    dropped_undo: bool,
}

enum Open {
    Ready(Opened),
    Hold(String),
}

fn open_save(text: &str) -> Open {
    let mut value: serde_json::Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(err) => return Open::Hold(format!("This save could not be read ({err}).")),
    };
    let Some(version) = value.get("version").and_then(|item| item.as_u64()) else {
        return Open::Hold("This save has no version.".into());
    };
    if version == 0 || version > u64::from(VERSION) {
        return Open::Hold(format!("This save is version {version}, which this table does not open."));
    }
    match serde_json::from_value(value.clone()) {
        Ok(save) => Open::Ready(Opened { save, dropped_undo: false }),
        Err(_) if version < u64::from(VERSION) && undo_present(&value) => {
            if let Some(object) = value.as_object_mut() {
                object.insert("undo".into(), serde_json::Value::Null);
            }
            match serde_json::from_value(value) {
                Ok(save) => Open::Ready(Opened { save, dropped_undo: true }),
                Err(retry) => Open::Hold(format!("This save could not be read ({retry}).")),
            }
        }
        Err(err) => Open::Hold(format!("This save could not be read ({err}).")),
    }
}

fn undo_present(value: &serde_json::Value) -> bool {
    value.get("undo").is_some_and(|item| !item.is_null())
}

/// Copy the file beside itself. The name includes a content hash, so a second
/// look at the same bytes does not replace the copy.
fn keep_backup(path: &Path) -> std::io::Result<PathBuf> {
    let bytes = std::fs::read(path)?;
    let mut hash = 0xcbf29ce484222325u64;
    for byte in &bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let name = path.file_name().and_then(|item| item.to_str()).unwrap_or("state.json");
    let dest = path.with_file_name(format!("{name}.bak-{hash:08x}"));
    if dest.is_file() {
        return Ok(dest);
    }
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&dest)?;
    std::io::Write::write_all(&mut file, &bytes)?;
    Ok(dest)
}

fn migration_notes(table: &Table, dropped_undo: bool) -> String {
    let mut lines = vec!["This table was saved by an older version. A copy of that file was kept.".to_string()];
    let mut scenes = Vec::new();
    if let Some(scene) = &table.scene {
        scenes.push(scene);
    }
    for shelf in &table.scenes {
        if let Some(scene) = &shelf.scene {
            scenes.push(scene);
        }
    }
    for campaign in &table.shelved {
        if let Some(scene) = &campaign.scene {
            scenes.push(scene);
        }
        for shelf in &campaign.scenes {
            if let Some(scene) = &shelf.scene {
                scenes.push(scene);
            }
        }
    }
    if scenes.is_empty() {
        lines.push("No map was on the table.".into());
    } else {
        let names = scenes.iter().map(|scene| scene.name.as_str()).collect::<Vec<_>>().join(", ");
        lines.push(format!("Opened {names}."));
    }
    for scene in &scenes {
        if scene.presentation.is_some() {
            lines.push(format!("{} still has an angled picture. It stays filed, and the table draws the fixed view.", scene.name));
        }
        if scene.image_file.is_some() && scene.layout.is_none() {
            let file = scene.image_file.as_deref().unwrap_or("a picture");
            lines.push(format!("{} still names its picture ({file}). That picture was not rebuilt into tiles.", scene.name));
        }
        if !scene.walls.is_empty() && scene.sight.is_empty() {
            lines.push(format!("{} has a wall with no recorded height. That wall is treated as 8 units tall.", scene.name));
        }
        if scene.layout.is_none() {
            lines.push(format!("{} has no height grid, so every square stays on the floor.", scene.name));
        }
    }
    let mut tokens = table.tokens.iter().collect::<Vec<_>>();
    for shelf in &table.scenes {
        tokens.extend(shelf.tokens.iter());
    }
    for token in tokens {
        if token.image_file.is_some() && token.cast_asset.is_none() {
            lines.push(format!("{} keeps a portrait and has no figure picture, so the figure stands in until you choose one.", token.name));
        }
    }
    if dropped_undo {
        lines.push("The last undo step could not be kept, so it was cleared.".into());
    }
    lines.join(" ")
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
    // Older shared exploration cannot identify which seat saw a cell. Preserve
    // it for the GM, and rebuild each seat's memory from that seat's own eyes.
    table.fog_memory = save.fog_memory;
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
    table.reveal_from_tokens();
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
        assert!(load_file(&mut table, &path).unwrap());
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
        let review = table.review.clone().unwrap_or_default();
        assert!(review.contains("older version"), "{review}");
        assert!(review.contains("Hall"), "{review}");
        assert!(review.contains("missing-map.png"), "{review}");
        assert!(review.contains("8 units"), "{review}");
        assert!(review.contains("Marker"), "{review}");
        assert!(review.contains("Cellar"), "{review}");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!user.ends_with("tests/fixtures/v1-mixed.json"));
    }

    #[test]
    fn a_future_save_is_kept_and_not_replaced() {
        let dir = std::env::temp_dir().join(format!("longtable-future-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let original = r#"{"version":99,"players":[],"scene":{"id":"hall","name":"Hall"},"tokens":[],"explored":{"width":1,"height":1,"bits":"AA=="},"rolls":[]}"#;
        std::fs::write(&path, original).unwrap();
        let mut table = Table::new();
        table.campaign.name = "Do not replace".into();
        let err = load_file(&mut table, &path).unwrap_err();
        assert!(err.to_string().contains("version 99"), "{err}");
        assert!(table.save_hold.is_some());
        assert!(write(&table, &dir).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        let copies = std::fs::read_dir(&dir).unwrap().filter_map(|item| item.ok()).filter(|item| {
            item.file_name().to_string_lossy().contains(".bak-")
        }).count();
        assert_eq!(copies, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_version_one_save_keeps_its_ids_and_a_copy() {
        let dir = std::env::temp_dir().join(format!("longtable-v1-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let original = r##"{
            "version":1,"players":[],"scene":null,
            "tokens":[{"id":"t","name":"Ada","x":1.5,"y":1.5,"image_file":null,"color":"#fff","owner_id":null,"hidden":false,"vision_radius":4,"grants_vision":true}],
            "explored":{"width":1,"height":1,"bits":"AA=="},
            "rolls":[],
            "campaign":{"id":"camp-1","name":"Kept","system_id":null},
            "characters":[{"id":"ada","name":"Ada","kind":"pc","owner_id":null,"numbers":{}}]
        }"##;
        std::fs::write(&path, original).unwrap();
        let mut table = Table::new();
        assert!(load_file(&mut table, &path).unwrap());
        assert!(table.save_hold.is_none());
        let review = table.review.clone().unwrap_or_default();
        assert!(review.contains("older version"), "{review}");
        assert!(review.contains("No map was on the table"), "{review}");
        table.players.push(crate::session::state::Player {
            id: "gm".into(),
            name: "Host".into(),
            role: crate::net::protocol::Role::Gm,
            color: "#fff".into(),
            connected: true,
        });
        table.players.push(crate::session::state::Player {
            id: "ada".into(),
            name: "Ada".into(),
            role: crate::net::protocol::Role::Player,
            color: "#fff".into(),
            connected: true,
        });
        let gm = crate::session::state::Viewer { player_id: "gm".into(), role: crate::net::protocol::Role::Gm, view_as: None, session_token: "g".into() };
        let seat = crate::session::state::Viewer { player_id: "ada".into(), role: crate::net::protocol::Role::Player, view_as: None, session_token: "a".into() };
        assert!(crate::session::permissions::snapshot(&table, &gm).review.unwrap().contains("older version"));
        assert!(crate::session::permissions::snapshot(&table, &seat).review.is_none());
        assert_eq!(table.campaign.id, "camp-1");
        assert_eq!(table.characters[0].id, "ada");
        assert_eq!(table.tokens[0].id, "t");
        write_to(&table, &path).unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("\"version\": 2"));
        assert!(written.contains("camp-1"));
        let backup = std::fs::read_dir(&dir).unwrap().filter_map(|item| item.ok()).find(|item| item.file_name().to_string_lossy().contains(".bak-")).unwrap();
        let kept = std::fs::read_to_string(backup.path()).unwrap();
        assert!(kept.contains("\"version\":1") || kept.contains("\"version\": 1"));
        assert!(kept.contains("camp-1"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_old_undo_step_is_cleared_and_the_table_still_opens() {
        let dir = std::env::temp_dir().join(format!("longtable-undo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let original = r##"{
            "version":1,"players":[],"scene":null,
            "tokens":[{"id":"t","name":"Ada","x":1.5,"y":1.5,"image_file":null,"color":"#fff","owner_id":null,"hidden":false,"vision_radius":4,"grants_vision":true}],
            "explored":{"width":1,"height":1,"bits":"AA=="},
            "rolls":[],
            "campaign":{"id":"camp-1","name":"Kept","system_id":null},
            "characters":[{"id":"ada","name":"Ada","kind":"pc","owner_id":null,"numbers":{}}],
            "fear":3,
            "strike":{"target_id":"ada","step":"Minor","marks":1},
            "undo":{"snapshot":{"width":4}}
        }"##;
        std::fs::write(&path, original).unwrap();
        let mut table = Table::new();
        assert!(load_file(&mut table, &path).unwrap());
        assert!(table.save_hold.is_none());
        assert!(table.undo.is_none());
        assert_eq!(table.fear, 3);
        assert_eq!(table.strike.as_ref().unwrap().target_id, "ada");
        assert_eq!(table.characters[0].id, "ada");
        let review = table.review.clone().unwrap_or_default();
        assert!(review.contains("could not be kept"), "{review}");
        assert!(review.contains("cleared"), "{review}");
        write_to(&table, &path).unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(!written.contains("snapshot"));
        let kept = std::fs::read_dir(&dir).unwrap().filter_map(|item| item.ok()).find(|item| item.file_name().to_string_lossy().contains(".bak-")).unwrap();
        assert!(std::fs::read_to_string(kept.path()).unwrap().contains("snapshot"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_old_undo_step_that_still_fits_is_kept() {
        let dir = std::env::temp_dir().join(format!("longtable-undo-keep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let original = r##"{
            "version":1,"players":[],"scene":null,
            "tokens":[],
            "explored":{"width":1,"height":1,"bits":"AA=="},
            "rolls":[],
            "undo":{"changes":[{"Fear":{"before":2}}]}
        }"##;
        std::fs::write(&path, original).unwrap();
        let mut table = Table::new();
        assert!(load_file(&mut table, &path).unwrap());
        let step = table.undo.clone().expect("undo kept");
        assert_eq!(step.changes.len(), 1);
        let review = table.review.clone().unwrap_or_default();
        assert!(!review.contains("could not be kept"), "{review}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_current_save_with_a_broken_undo_stays_on_hold() {
        let dir = std::env::temp_dir().join(format!("longtable-undo-hold-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let original = r#"{"version":2,"players":[],"scene":null,"tokens":[],"explored":{"width":1,"height":1,"bits":"AA=="},"rolls":[],"undo":{"snapshot":true}}"#;
        std::fs::write(&path, original).unwrap();
        let mut table = Table::new();
        table.campaign.name = "Do not replace".into();
        let err = load_file(&mut table, &path).unwrap_err();
        assert!(err.to_string().contains("could not be read"), "{err}");
        assert!(table.save_hold.is_some());
        assert!(write(&table, &dir).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
