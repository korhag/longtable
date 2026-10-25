// ------------------------------------------------------------
// Module: storage
// Purpose: Write the table to data/state.json shortly after it
//          changes, and read it back on the next boot. Sockets
//          and session tokens are not stored.
// ------------------------------------------------------------

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::map::Scene;
use crate::session::state::{CampaignShelf, Player, SceneShelf, StoredRoll, Table, Token};
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
    let save = SaveFile {
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
    };
    std::fs::create_dir_all(data_dir)?;
    let path = data_dir.join("state.json");
    let temp = data_dir.join("state.json.tmp");
    let file = std::fs::File::create(&temp)?;
    serde_json::to_writer_pretty(file, &save).map_err(std::io::Error::other)?;
    std::fs::rename(&temp, &path)?;
    Ok(())
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
        let _ = std::fs::remove_dir_all(&dir);
    }
}

fn backup(path: &Path) {
    let mut bak = PathBuf::from(path);
    bak.set_extension("json.bak");
    if let Err(err) = std::fs::rename(path, &bak) {
        tracing::warn!("could not move the unreadable save aside: {err}");
    }
}
