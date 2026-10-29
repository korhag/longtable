// ------------------------------------------------------------
// Module: attack
// Purpose: The formula and the verdict for a targeted attack.
//          A player uses the trait on their weapon. An adversary
//          uses their attack modifier and a d20. Meeting the
//          defense hits, and so does a duality critical.
// ------------------------------------------------------------

use super::super::Character;
use super::super::CharacterKind;

const TRAITS: &[(&str, &str)] = &[
    ("agility", "agility"),
    ("strength", "strength"),
    ("finesse", "finesse"),
    ("instinct", "instinct"),
    ("presence", "presence"),
    ("knowledge", "knowledge"),
];

pub fn attack_formula(character: &Character) -> String {
    if character.kind == CharacterKind::Adversary {
        return format!("1d20{}", signed(number(character, "attack")));
    }
    format!("1d12h+1d12f{}", signed(trait_mod(character)))
}

pub fn damage_formula(character: &Character) -> String {
    let key = if character.kind == CharacterKind::Adversary { "atkdamage" } else { "damage" };
    let text = character.texts.get(key).map(|text| dice_text(text)).unwrap_or_default();
    if text.is_empty() { "1d8".into() } else { text }
}

pub fn defense_of(character: &Character) -> (&'static str, i32) {
    if character.kind == CharacterKind::Adversary {
        ("Difficulty", character.numbers.get("difficulty").copied().unwrap_or(11))
    } else {
        ("Evasion", character.numbers.get("evasion").copied().unwrap_or(10))
    }
}

/// A roll hits when it meets the defense. Matching Hope and Fear is a critical, and that hits too.
pub fn attack_hits(total: i32, defense: i32, critical: bool) -> bool {
    critical || total >= defense
}

fn trait_mod(character: &Character) -> i32 {
    let primary = character.texts.get("primary").map(String::as_str).unwrap_or("");
    let parts: Vec<String> = primary.split('·').map(|part| part.trim().to_ascii_lowercase()).collect();
    for (id, name) in TRAITS {
        if parts.iter().any(|part| part == name) {
            return number(character, id);
        }
    }
    0
}

fn number(character: &Character, id: &str) -> i32 {
    character.numbers.get(id).copied().unwrap_or(0)
}

fn signed(value: i32) -> String {
    if value > 0 {
        format!("+{value}")
    } else if value < 0 {
        value.to_string()
    } else {
        String::new()
    }
}

fn dice_text(text: &str) -> String {
    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();
    let stripped = [" direct phy", " direct mag", " phy", " mag"]
        .iter()
        .find_map(|suffix| lower.strip_suffix(suffix).map(|rest| trimmed[..rest.len()].trim()))
        .unwrap_or(trimmed);
    if !stripped.is_empty() && stripped.chars().all(|ch| ch.is_ascii_digit()) {
        format!("flat:{stripped}")
    } else {
        stripped.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn character(kind: CharacterKind, numbers: BTreeMap<String, i32>, primary: &str) -> Character {
        let mut texts = BTreeMap::new();
        if !primary.is_empty() {
            texts.insert("primary".into(), primary.into());
        }
        Character {
            id: "c".into(),
            name: "Ada".into(),
            kind,
            owner_id: None,
            system_id: None,
            numbers,
            texts,
        }
    }

    #[test]
    fn a_weapon_trait_is_the_attack_modifier() {
        let mut numbers = BTreeMap::new();
        numbers.insert("instinct".into(), 2);
        let sheet = character(CharacterKind::Pc, numbers, "Dualstaff · Instinct · Far · 1d6+3 mag");
        assert_eq!(attack_formula(&sheet), "1d12h+1d12f+2");
    }

    #[test]
    fn an_adversary_rolls_a_d20() {
        let mut numbers = BTreeMap::new();
        numbers.insert("attack".into(), -1);
        let foe = character(CharacterKind::Adversary, numbers, "");
        assert_eq!(attack_formula(&foe), "1d20-1");
    }

    #[test]
    fn meeting_the_defense_hits_and_a_critical_hits_short() {
        assert!(attack_hits(10, 10, false));
        assert!(!attack_hits(9, 10, false));
        assert!(attack_hits(4, 10, true));
    }

    #[test]
    fn damage_text_drops_a_trailing_type() {
        let mut texts = BTreeMap::new();
        texts.insert("atkdamage".into(), "1d8 phy".into());
        let foe = Character {
            id: "f".into(),
            name: "Bandit".into(),
            kind: CharacterKind::Adversary,
            owner_id: None,
            system_id: None,
            numbers: BTreeMap::new(),
            texts,
        };
        assert_eq!(damage_formula(&foe), "1d8");
    }

    #[test]
    fn a_flat_damage_number_is_a_flat_roll() {
        let mut texts = BTreeMap::new();
        texts.insert("atkdamage".into(), "2 phy".into());
        let foe = Character {
            id: "f".into(),
            name: "Lackey".into(),
            kind: CharacterKind::Adversary,
            owner_id: None,
            system_id: None,
            numbers: BTreeMap::new(),
            texts,
        };
        assert_eq!(damage_formula(&foe), "flat:2");
    }
}
