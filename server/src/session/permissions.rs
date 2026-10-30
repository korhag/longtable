// ------------------------------------------------------------
// Module: permissions
// Purpose: Turn the full table into the view one connection is
//          allowed to see. Hidden tokens, secret rolls, and
//          unexplored doors never leave the server.
// ------------------------------------------------------------

use crate::map::Vec2;
use crate::net::protocol::{
    DieResult, DoorView, FogMode, FogView, LightView, Listing, PlayerInfo, Presentation, PresentationCapabilities, RollLogEntry,
    RollVisibility, SceneView, SegmentView, Snapshot, StrikeOffer, TokenKind, TokenView, Figure,
};
use crate::systems::{Character, CharacterKind};
use crate::vision::{point_in_poly, CELLS_PER_SQUARE};

use super::state::{Table, Token, Viewer};
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

fn filed_notice(presentation: &crate::map::Prerendered) -> Presentation {
    let mut warnings = vec![crate::map::presentation::FILED_VIEW.to_string()];
    warnings.extend(presentation.warnings.iter().cloned());
    Presentation {
        active_lighting: String::new(),
        default_view: String::new(),
        view_order: Vec::new(),
        lightings: Vec::new(),
        views: Vec::new(),
        doors: Vec::new(),
        capabilities: PresentationCapabilities {
            depth: false,
            doors: false,
            move_blockers: false,
            walls: false,
            lights: false,
            snow: false,
        },
        warnings,
    }
}

pub fn snapshot(table: &Table, viewer: &Viewer) -> Snapshot {
    let you = player_info(table, &viewer.player_id).expect("viewer is seated");
    let players = table.players.iter().map(|player| player_info(table, &player.id).unwrap()).collect();
    let sight = sight_of(viewer);
    let polygons = match &sight {
        Sight::Gm => Vec::new(),
        Sight::Player { .. } if table.uses_height_sight() => Vec::new(),
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
            presentation: scene.presentation.as_ref().map(filed_notice),
            schema_version: scene.schema_version,
            visual_profile: scene.visual_profile.clone(),
        }),
        tokens: table
            .tokens
            .iter()
            .filter(|token| match &sight {
                Sight::Gm => true,
                Sight::Player { id } => token_visible(table, token, id, &polygons),
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
                cast_asset: resolved_cast(table, token),
                facing: token.facing.clone(),
                footprint: Some(token.footprint),
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
            visible: match &sight {
                Sight::Gm => None,
                Sight::Player { id } => table.visible_bits(id),
            },
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
                        contest: roll.contest.clone(),
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
                        contest: None,
                    }
                }
            })
            .collect(),
        view_as: if viewer.role == Role::Gm { viewer.view_as.clone() } else { None },
        campaign: table.campaign.clone(),
        characters: visible_characters(table, viewer),
        campaigns: if matches!(sight, Sight::Gm) { campaign_listings(table) } else { Vec::new() },
        scenes: if matches!(sight, Sight::Gm) { scene_listings(table) } else { Vec::new() },
        fear: table.fear,
        figures: faces(table, &sight, &polygons),
        can_undo: table.undo.is_some(),
        strike: strike_offer(table, viewer),
        talk: table.talk.clone(),
        catalog_revision: table.catalog_revision.clone(),
        notice: table.save_hold.clone(),
        review: if viewer.role == Role::Gm { table.review.clone() } else { None },
    }
}

fn strike_offer(table: &Table, viewer: &Viewer) -> Option<StrikeOffer> {
    let pending = table.strike.as_ref()?;
    let character = table.characters.iter().find(|character| character.id == pending.target_id)?;
    let gm = viewer.role == Role::Gm && viewer.view_as.is_none();
    let owner = character.owner_id.as_deref() == Some(viewer.player_id.as_str());
    if !gm && !owner {
        return None;
    }
    Some(StrikeOffer {
        target_id: pending.target_id.clone(),
        target_name: character.name.clone(),
        step: pending.step.clone(),
        marks: pending.marks,
        with_armor: (pending.marks - 1).max(0),
    })
}

const HEALTH: [&str; 5] = ["Uninjured", "Slightly Wounded", "Wounded", "Badly Wounded", "Near Death"];
const STRESS: [&str; 5] = ["Clear", "Uneasy", "Strained", "Stressed", "Spent"];

fn faces(table: &Table, sight: &Sight, polygons: &[Vec<Vec2>]) -> Vec<Figure> {
    table
        .tokens
        .iter()
        .filter(|token| match sight {
            Sight::Gm => true,
            Sight::Player { id } => token_visible(table, token, id, polygons),
        })
        .filter_map(|token| figure_for(table, token))
        .collect()
}

fn figure_for(table: &Table, token: &Token) -> Option<Figure> {
    let id = token.character_id.as_ref()?;
    let character = table.characters.iter().find(|character| &character.id == id)?;
    let number = |key: &str| character.numbers.get(key).copied().unwrap_or(0);
    Some(Figure {
        token_id: token.id.clone(),
        name: token.name.clone(),
        kind: character.kind,
        health: track_word(number("hp"), number("hpmax"), &HEALTH),
        stress: track_word(number("stress"), number("stressmax"), &STRESS),
        hope: if character.kind == CharacterKind::Pc { Some(number("hope")) } else { None },
    })
}

/// Name a mark track by quarters. The last quarter, including a full track, is the final word.
fn track_word(marked: i32, max: i32, names: &[&str; 5]) -> String {
    if max <= 0 || marked <= 0 {
        return names[0].to_string();
    }
    if marked >= max {
        return names[4].to_string();
    }
    let step = ((marked as f64 / max as f64) * 4.0).ceil() as usize;
    names[step.clamp(1, 4)].to_string()
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

fn resolved_cast(table: &Table, token: &super::state::Token) -> Option<String> {
    if token.cast_override {
        return token.cast_asset.clone().filter(|asset| !asset.is_empty());
    }
    token
        .character_id
        .as_ref()
        .and_then(|id| table.characters.iter().find(|character| &character.id == id))
        .and_then(|character| character.cast_asset_id.clone())
        .or_else(|| token.cast_asset.clone())
        .filter(|asset| !asset.is_empty())
}

fn token_visible(table: &super::state::Table, token: &super::state::Token, viewer_id: &str, polygons: &[Vec<Vec2>]) -> bool {
    if token.hidden {
        return false;
    }
    if token.owner_id.as_deref() == Some(viewer_id) {
        return true;
    }
    if table.uses_height_sight() {
        return table.viewer_sees(viewer_id, token);
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
            character_id: Some("lurker".into()),
            cast_override: false,
            cast_asset: None,
            facing: "south".into(),
            footprint: [1, 1],
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
            character_id: Some("ada-sheet".into()),
            cast_override: false,
            cast_asset: None,
            facing: "south".into(),
            footprint: [1, 1],
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
            contest: None,
        });
        table.characters.push(Character {
            id: "ada-sheet".into(),
            name: "Ada".into(),
            kind: CharacterKind::Pc,
            owner_id: Some("p1".into()),
            system_id: None,
            numbers: [("hp", 2), ("hpmax", 6), ("stress", 0), ("stressmax", 6), ("hope", 2)]
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
            texts: Default::default(),
            cast_asset_id: None,
            });
        table.characters.push(Character {
            id: "lurker".into(),
            name: "Lurker".into(),
            kind: CharacterKind::Adversary,
            owner_id: None,
            system_id: None,
            numbers: [("hp", 5), ("hpmax", 5), ("stress", 3), ("stressmax", 3)]
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
            texts: [("feature".into(), "SECRET-FEATURE-XYZ".into())].into_iter().collect(),
            cast_asset_id: None,
            });

        let player_view = snapshot(&table, &viewer(Role::Player, "p1"));
        assert!(player_view.tokens.iter().all(|token| token.id != "secret"));
        assert!(player_view.tokens.iter().any(|token| token.id == "ada"));
        assert!(player_view.rolls[0].concealed);
        assert!(player_view.rolls[0].total.is_none());
        assert!(player_view.rolls[0].outcome.is_none());
        assert!(player_view.characters.iter().any(|character| character.id == "ada-sheet"));
        assert!(player_view.characters.iter().all(|character| character.id != "lurker"));
        let player_json = serde_json::to_string(&player_view).unwrap();
        assert!(!player_json.contains("SECRET-FEATURE-XYZ"));
        assert_eq!(player_view.figures.len(), 1);
        assert_eq!(player_view.figures[0].token_id, "ada");
        assert_eq!(player_view.figures[0].health, "Wounded");
        assert_eq!(player_view.figures[0].stress, "Clear");
        assert_eq!(player_view.figures[0].hope, Some(2));
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
        let lurker = gm_view.figures.iter().find(|figure| figure.token_id == "secret").unwrap();
        assert_eq!(lurker.health, "Near Death");
        assert_eq!(lurker.stress, "Spent");
        assert_eq!(lurker.hope, None);
    }

    #[test]
    fn a_player_does_not_receive_an_adversary_outside_sight() {
        use crate::map::{Scene, Segment, DIMETRIC_PROFILE};

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
        table.scene = Some(Scene {
            id: "hall".into(),
            name: "Hall".into(),
            width: 20.0,
            height: 20.0,
            image_file: None,
            walls: vec![Segment { a: Vec2::new(5.0, 0.0), b: Vec2::new(5.0, 10.0) }],
            doors: Vec::new(),
            lights: Vec::new(),
            layout: None,
            move_blockers: Vec::new(),
            sight: Vec::new(),
            presentation: None,
            schema_version: 2,
            visual_profile: DIMETRIC_PROFILE.to_string(),
        });
        let token = |id: &str, name: &str, x: f32, y: f32, owner: Option<&str>, radius: f32, grants: bool| Token {
            id: id.into(),
            name: name.into(),
            x,
            y,
            image_file: None,
            color: "#d4a656".into(),
            owner_id: owner.map(str::to_string),
            hidden: false,
            vision_radius: radius,
            grants_vision: grants,
            character_id: None,
            cast_override: false,
            cast_asset: Some("gate/bandit".into()),
            facing: "south".into(),
            footprint: [1, 1],
        };
        table.tokens.push(token("ada", "Ada", 2.0, 5.0, Some("p1"), 20.0, true));
        table.tokens.push(token("near", "Near", 3.0, 5.0, None, 0.0, false));
        table.tokens.push(token("shade", "Shade", 7.0, 5.0, None, 0.0, false));
        table.tokens.push(token("far", "Far", 40.0, 40.0, None, 0.0, false));

        let player_view = snapshot(&table, &viewer(Role::Player, "p1"));
        let names = player_view.tokens.iter().map(|token| token.name.as_str()).collect::<Vec<_>>();
        assert!(names.contains(&"Ada") && names.contains(&"Near"), "{names:?}");
        assert!(!names.contains(&"Shade") && !names.contains(&"Far"), "{names:?}");
        let player_json = serde_json::to_string(&player_view).unwrap();
        assert!(!player_json.contains("Shade") && !player_json.contains("\"Far\""));
        assert!(player_view.figures.iter().all(|figure| figure.name != "Shade"));

        let gm_view = snapshot(&table, &viewer(Role::Gm, "gm"));
        assert!(gm_view.tokens.iter().any(|token| token.name == "Shade"));
        let mut eyes = viewer(Role::Gm, "gm");
        eyes.view_as = Some("p1".into());
        let as_player = snapshot(&table, &eyes);
        assert!(as_player.tokens.iter().all(|token| token.name != "Shade" && token.name != "Far"));
    }

    #[test]
    fn a_high_observer_sees_over_a_hedge() {
        use crate::map::layout::encode_cells;
        use crate::map::{MapLayout, Scene, SightBlock, Vec2, DIMETRIC_PROFILE};

        let mut levels = vec![0_u8; 64];
        levels[1 * 8 + 1] = 2;
        let mut layout = MapLayout::blank(8, 8, "core/grass");
        layout.levels = Some(encode_cells(&levels));
        let mut table = Table::new();
        table.players.push(crate::session::state::Player {
            id: "p1".into(),
            name: "Ada".into(),
            role: Role::Player,
            color: "#7eb8a2".into(),
            connected: true,
        });
        table.scene = Some(Scene {
            id: "rise".into(),
            name: "Rise".into(),
            width: 8.0,
            height: 8.0,
            image_file: None,
            walls: Vec::new(),
            doors: Vec::new(),
            lights: Vec::new(),
            layout: Some(layout),
            move_blockers: Vec::new(),
            sight: vec![SightBlock { a: Vec2::new(4.0, 0.0), b: Vec2::new(4.0, 8.0), base: 0.0, top: 0.5 }],
            presentation: None,
            schema_version: 2,
            visual_profile: DIMETRIC_PROFILE.to_string(),
        });
        let token = |id: &str, name: &str, x: f32, y: f32, grants: bool| crate::session::state::Token {
            id: id.into(),
            name: name.into(),
            x,
            y,
            image_file: None,
            color: "#d4a656".into(),
            owner_id: None,
            hidden: false,
            vision_radius: 20.0,
            grants_vision: grants,
            character_id: None,
            cast_override: false,
            cast_asset: None,
            facing: "south".into(),
            footprint: [1, 1],
        };
        table.tokens.push(token("ada", "Ada", 1.5, 1.5, true));
        table.tokens.push(token("far", "Far", 6.5, 1.5, false));
        table.reveal_from_tokens();
        let player = viewer(Role::Player, "p1");
        let seen = snapshot(&table, &player);
        assert!(seen.tokens.iter().any(|token| token.name == "Far"), "a high eye should clear the hedge");
        assert!(seen.fog.visible.is_some());
        if let Some(scene) = &mut table.scene {
            scene.sight[0].top = 4.0;
        }
        let hidden = snapshot(&table, &player);
        assert!(hidden.tokens.iter().all(|token| token.name != "Far"), "a tall wall still hides the far figure");
    }

    #[test]
    fn quarters_name_both_tracks() {
        let health = ["Uninjured", "Slightly Wounded", "Wounded", "Badly Wounded", "Near Death"];
        assert_eq!(track_word(0, 8, &health), "Uninjured");
        assert_eq!(track_word(1, 8, &health), "Slightly Wounded");
        assert_eq!(track_word(2, 8, &health), "Slightly Wounded");
        assert_eq!(track_word(3, 8, &health), "Wounded");
        assert_eq!(track_word(4, 8, &health), "Wounded");
        assert_eq!(track_word(5, 8, &health), "Badly Wounded");
        assert_eq!(track_word(6, 8, &health), "Badly Wounded");
        assert_eq!(track_word(7, 8, &health), "Near Death");
        assert_eq!(track_word(8, 8, &health), "Near Death");
        assert_eq!(track_word(0, 0, &health), "Uninjured");
        let stress = ["Clear", "Uneasy", "Strained", "Stressed", "Spent"];
        assert_eq!(track_word(0, 6, &stress), "Clear");
        assert_eq!(track_word(6, 6, &stress), "Spent");
    }

    #[test]
    fn a_public_fear_roll_fills_the_pool_and_stops_at_twelve() {
        use crate::net::protocol::{ClientMsg, RollEvent};

        let mut table = Table::new();
        table.campaign.system_id = Some("daggerheart".into());
        let fear = RollEvent {
            id: "r".into(),
            roller_id: "gm".into(),
            roller_name: "Mara".into(),
            formula: "1d12h+1d12f".into(),
            label: None,
            visibility: RollVisibility::Public,
            modifier: 0,
            total: 10,
            play_at: 0,
            keyframe_dt: 0.0,
            dice: Vec::new(),
            outcome: Some(RollOutcome { label: "with Fear".into(), tone: "fear".into() }),
            contest: None,
        };
        for _ in 0..13 {
            table.commit_roll(&fear);
        }
        assert_eq!(table.fear, 12);
        table.command(&viewer(Role::Gm, "gm"), ClientMsg::AdjustFear { delta: -1 }, None).unwrap();
        assert_eq!(table.fear, 11);
        assert!(table.command(&viewer(Role::Player, "p1"), ClientMsg::AdjustFear { delta: 1 }, None).is_err());
        assert_eq!(table.fear, 11);

        table.campaign.system_id = None;
        table.commit_roll(&fear);
        assert_eq!(table.fear, 11);
        table.campaign.system_id = Some("daggerheart".into());

        let mut secret = fear.clone();
        secret.visibility = RollVisibility::GmSecret;
        table.commit_roll(&secret);
        assert_eq!(table.fear, 11);

        let mut hope = fear.clone();
        hope.outcome = Some(RollOutcome { label: "with Hope".into(), tone: "hope".into() });
        table.commit_roll(&hope);
        assert_eq!(table.fear, 11);
    }

    #[test]
    fn a_whispered_roll_is_visible_to_the_roller_and_the_gm() {
        assert!(can_see_roll(&viewer(Role::Player, "p1"), RollVisibility::ToGm, "p1"));
        assert!(can_see_roll(&viewer(Role::Gm, "gm"), RollVisibility::ToGm, "p1"));
        assert!(!can_see_roll(&viewer(Role::Player, "p2"), RollVisibility::ToGm, "p1"));
        assert!(!can_see_roll(&viewer(Role::Player, "p2"), RollVisibility::GmSecret, "gm"));
    }
}
