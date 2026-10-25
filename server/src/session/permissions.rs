// ------------------------------------------------------------
// Module: permissions
// Purpose: Turn the full table into the view one connection is
//          allowed to see. Hidden tokens, secret rolls, and
//          unexplored doors never leave the server.
// ------------------------------------------------------------

use crate::map::Vec2;
use crate::net::protocol::{
    DieResult, DoorView, FogMode, FogView, LightView, Listing, PlayerInfo, RollLogEntry, RollVisibility, SceneView,
    SegmentView, Snapshot, TokenKind, TokenView,
};
use crate::systems::Character;
use crate::vision::{point_in_poly, CELLS_PER_SQUARE};

use super::state::{Table, Viewer};
use crate::net::protocol::Role;

pub fn can_see_roll(viewer: &Viewer, visibility: RollVisibility, roller_id: &str) -> bool {
    match visibility {
        RollVisibility::Public => true,
        RollVisibility::ToGm => viewer.role == Role::Gm || viewer.player_id == roller_id,
        RollVisibility::GmSecret => viewer.role == Role::Gm,
    }
}

pub fn conceal_notice(visibility: RollVisibility, roller_name: &str) -> String {
    match visibility {
        RollVisibility::GmSecret => "The GM rolls in secret…".into(),
        RollVisibility::ToGm => format!("{roller_name} rolls to the GM…"),
        RollVisibility::Public => String::new(),
    }
}

pub fn snapshot(table: &Table, viewer: &Viewer) -> Snapshot {
    let you = player_info(table, &viewer.player_id).expect("viewer is seated");
    let players = table.players.iter().map(|player| player_info(table, &player.id).unwrap()).collect();
    let sight = sight_of(viewer);
    let polygons = match &sight {
        Sight::Gm => Vec::new(),
        Sight::Player { id } => table.vision_polygons(id),
    };

    Snapshot {
        you,
        players,
        scene: table.scene.as_ref().map(|scene| SceneView {
            id: scene.id.clone(),
            name: scene.name.clone(),
            width: scene.width,
            height: scene.height,
            image_url: scene.image_file.as_ref().map(|_| {
                format!("/api/maps/{}/image?session={}", scene.id, viewer.session_token)
            }),
            walls: if matches!(sight, Sight::Gm) {
                scene
                    .walls
                    .iter()
                    .map(|wall| SegmentView { a: [wall.a.x, wall.a.y], b: [wall.b.x, wall.b.y] })
                    .collect()
            } else {
                Vec::new()
            },
            lights: if matches!(sight, Sight::Gm) {
                scene
                    .lights
                    .iter()
                    .map(|light| LightView {
                        x: light.x,
                        y: light.y,
                        range: light.range,
                        color: light.color.clone(),
                    })
                    .collect()
            } else {
                Vec::new()
            },
            layout: scene.layout.clone(),
        }),
        tokens: table
            .tokens
            .iter()
            .filter(|token| match &sight {
                Sight::Gm => true,
                Sight::Player { id } => token_visible(token, id, &polygons),
            })
            .map(|token| TokenView {
                id: token.id.clone(),
                name: token.name.clone(),
                x: token.x,
                y: token.y,
                kind: TokenKind::Disc,
                image_url: token
                    .image_file
                    .as_ref()
                    .map(|_| format!("/api/tokens/{}/image?session={}", token.id, viewer.session_token)),
                color: token.color.clone(),
                owner_id: token.owner_id.clone(),
                hidden: matches!(sight, Sight::Gm) && token.hidden,
                vision_radius: token.vision_radius,
                grants_vision: token.grants_vision,
                character_id: token.character_id.clone(),
            })
            .collect(),
        doors: table
            .scene
            .as_ref()
            .map(|scene| {
                scene
                    .doors
                    .iter()
                    .filter(|door| match &sight {
                        Sight::Gm => true,
                        Sight::Player { .. } => {
                            let mid_x = (door.a.x + door.b.x) * 0.5;
                            let mid_y = (door.a.y + door.b.y) * 0.5;
                            table.explored_at(mid_x, mid_y)
                        }
                    })
                    .map(|door| DoorView {
                        id: door.id.clone(),
                        a: [door.a.x, door.a.y],
                        b: [door.b.x, door.b.y],
                        open: door.open,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        fog: FogView {
            cells_per_square: CELLS_PER_SQUARE,
            width: table.explored.width,
            height: table.explored.height,
            explored: table.explored.to_base64(),
            visible_polygons: polygons
                .iter()
                .map(|polygon| polygon.iter().map(|point| [point.x, point.y]).collect())
                .collect(),
            mode: if matches!(sight, Sight::Gm) { FogMode::Gm } else { FogMode::Player },
        },
        rolls: table
            .rolls
            .iter()
            .map(|roll| {
                if can_see_roll(viewer, roll.visibility, &roll.roller_id) {
                    RollLogEntry {
                        id: roll.id.clone(),
                        roller_name: roll.roller_name.clone(),
                        formula: Some(roll.formula.clone()),
                        label: roll.label.clone(),
                        visibility: roll.visibility,
                        modifier: roll.modifier,
                        total: Some(roll.total),
                        dice: roll.dice.clone(),
                        concealed: false,
                        notice: None,
                        created_at: roll.created_at,
                        outcome: roll.outcome.clone(),
                    }
                } else {
                    RollLogEntry {
                        id: roll.id.clone(),
                        roller_name: roll.roller_name.clone(),
                        formula: None,
                        label: None,
                        visibility: roll.visibility,
                        modifier: 0,
                        total: None,
                        dice: Vec::<DieResult>::new(),
                        concealed: true,
                        notice: Some(conceal_notice(roll.visibility, &roll.roller_name)),
                        created_at: roll.created_at,
                        outcome: None,
                    }
                }
            })
            .collect(),
        view_as: if viewer.role == Role::Gm { viewer.view_as.clone() } else { None },
        campaign: table.campaign.clone(),
        characters: visible_characters(table, viewer),
        campaigns: if matches!(sight, Sight::Gm) { campaign_listings(table) } else { Vec::new() },
        scenes: if matches!(sight, Sight::Gm) { scene_listings(table) } else { Vec::new() },
    }
}

fn campaign_listings(table: &Table) -> Vec<Listing> {
    let mut listings = vec![Listing { id: table.campaign.id.clone(), name: table.campaign.name.clone() }];
    for shelf in &table.shelved {
        listings.push(Listing { id: shelf.campaign.id.clone(), name: shelf.campaign.name.clone() });
    }
    listings
}

fn scene_listings(table: &Table) -> Vec<Listing> {
    let mut listings = Vec::new();
    if let Some(scene) = &table.scene {
        listings.push(Listing { id: scene.id.clone(), name: scene.name.clone() });
    }
    for shelf in &table.scenes {
        listings.push(Listing { id: shelf.id.clone(), name: shelf.name.clone() });
    }
    listings
}

enum Sight {
    Gm,
    Player { id: String },
}

fn sight_of(viewer: &Viewer) -> Sight {
    if viewer.role == Role::Gm {
        if let Some(id) = &viewer.view_as {
            Sight::Player { id: id.clone() }
        } else {
            Sight::Gm
        }
    } else {
        Sight::Player { id: viewer.player_id.clone() }
    }
}

fn token_visible(token: &super::state::Token, viewer_id: &str, polygons: &[Vec<Vec2>]) -> bool {
    if token.hidden {
        return false;
    }
    if token.owner_id.as_deref() == Some(viewer_id) {
        return true;
    }
    let point = Vec2::new(token.x, token.y);
    polygons.iter().any(|polygon| point_in_poly(point, polygon))
}

fn visible_characters(table: &Table, viewer: &Viewer) -> Vec<Character> {
    let eyes = if viewer.role == Role::Gm && viewer.view_as.is_none() {
        None
    } else {
        Some(viewer.view_as.as_deref().unwrap_or(viewer.player_id.as_str()))
    };
    table
        .characters
        .iter()
        .filter(|character| eyes.is_none_or(|id| character.owner_id.as_deref() == Some(id)))
        .cloned()
        .collect()
}

fn player_info(table: &Table, id: &str) -> Option<PlayerInfo> {
    table.players.iter().find(|player| player.id == id).map(|player| PlayerInfo {
        id: player.id.clone(),
        name: player.name.clone(),
        role: player.role,
        color: player.color.clone(),
        connected: player.connected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::protocol::Role;
    use crate::session::state::{StoredRoll, Token};
    use crate::systems::{Character, CharacterKind, RollOutcome};

    fn viewer(role: Role, id: &str) -> Viewer {
        Viewer {
            player_id: id.into(),
            role,
            view_as: None,
            session_token: "test".into(),
        }
    }

    #[test]
    fn players_do_not_receive_hidden_tokens_or_secret_totals() {
        let mut table = Table::new();
        table.players.push(crate::session::state::Player {
            id: "gm".into(),
            name: "Mara".into(),
            role: Role::Gm,
            color: "#d4a656".into(),
            connected: true,
        });
        table.players.push(crate::session::state::Player {
            id: "p1".into(),
            name: "Ada".into(),
            role: Role::Player,
            color: "#7eb8a2".into(),
            connected: true,
        });
        table.tokens.push(Token {
            id: "secret".into(),
            name: "Lurker".into(),
            x: 1.5,
            y: 1.5,
            image_file: None,
            color: "#d4a656".into(),
            owner_id: None,
            hidden: true,
            vision_radius: 6.0,
            grants_vision: false,
            character_id: None,
        });
        table.tokens.push(Token {
            id: "ada".into(),
            name: "Ada".into(),
            x: 2.5,
            y: 2.5,
            image_file: None,
            color: "#7eb8a2".into(),
            owner_id: Some("p1".into()),
            hidden: false,
            vision_radius: 8.0,
            grants_vision: true,
            character_id: None,
        });
        table.rolls.push(StoredRoll {
            id: "r".into(),
            roller_id: "gm".into(),
            roller_name: "Mara".into(),
            formula: "1d20".into(),
            label: None,
            visibility: RollVisibility::GmSecret,
            modifier: 0,
            total: 17,
            dice: vec![DieResult { sides: 20, value: 17, sign: 1, tag: None }],
            created_at: 1,
            outcome: Some(RollOutcome { label: "with Hope".into(), tone: "hope".into() }),
        });
        table.characters.push(Character {
            id: "ada-sheet".into(),
            name: "Ada".into(),
            kind: CharacterKind::Pc,
            owner_id: Some("p1".into()),
            system_id: None,
            numbers: Default::default(),
            texts: Default::default(),
        });
        table.characters.push(Character {
            id: "lurker".into(),
            name: "Lurker".into(),
            kind: CharacterKind::Adversary,
            owner_id: None,
            system_id: None,
            numbers: Default::default(),
            texts: Default::default(),
        });

        let player_view = snapshot(&table, &viewer(Role::Player, "p1"));
        assert!(player_view.tokens.iter().all(|token| token.id != "secret"));
        assert!(player_view.tokens.iter().any(|token| token.id == "ada"));
        assert!(player_view.rolls[0].concealed);
        assert!(player_view.rolls[0].total.is_none());
        assert!(player_view.rolls[0].outcome.is_none());
        assert!(player_view.characters.iter().any(|character| character.id == "ada-sheet"));
        assert!(player_view.characters.iter().all(|character| character.id != "lurker"));
        assert!(player_view.scene.is_none() || player_view.scene.as_ref().unwrap().walls.is_empty());

        let gm_view = snapshot(&table, &viewer(Role::Gm, "gm"));
        assert!(gm_view.tokens.iter().any(|token| token.id == "secret" && token.hidden));
        assert_eq!(gm_view.rolls[0].total, Some(17));
        assert_eq!(gm_view.rolls[0].outcome.as_ref().map(|outcome| outcome.tone.as_str()), Some("hope"));
        assert_eq!(gm_view.characters.len(), 2);
        assert!(player_view.campaigns.is_empty());
        assert!(player_view.scenes.is_empty());
        assert_eq!(gm_view.campaigns.len(), 1);
        assert!(gm_view.scenes.is_empty());
    }

    #[test]
    fn a_whispered_roll_is_visible_to_the_roller_and_the_gm() {
        assert!(can_see_roll(&viewer(Role::Player, "p1"), RollVisibility::ToGm, "p1"));
        assert!(can_see_roll(&viewer(Role::Gm, "gm"), RollVisibility::ToGm, "p1"));
        assert!(!can_see_roll(&viewer(Role::Player, "p2"), RollVisibility::ToGm, "p1"));
        assert!(!can_see_roll(&viewer(Role::Player, "p2"), RollVisibility::GmSecret, "gm"));
    }
}
