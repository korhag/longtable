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
    if text.is_empty() {
        "1d8".into()
    } else if character.kind != CharacterKind::Adversary && text.starts_with('d') {
        // Equipment's bare d8 notation uses the wielder's Proficiency. Explicit
        // counts (2d8, @proficiency d8, spell damage) already state their dice.
        format!("{}{}", number(character, "proficiency").clamp(1, 6), text)
    } else {
        text
    }
}

pub fn deals_direct_damage(character: &Character) -> bool {
    let key = if character.kind == CharacterKind::Adversary { "atkdamage" } else { "damage" };
    character.texts.get(key).is_some_and(|text| text.split_whitespace().any(|word| word.eq_ignore_ascii_case("direct")))
}

/// Critical hits add the maximum of the damage dice, without doubling flat bonuses.
pub fn critical_damage_formula(formula: &str) -> Result<String, String> {
    let parsed = crate::dice::parse_formula(formula).map_err(|error| error.to_string())?;
    let maximum: i32 = parsed.groups.iter()
        .map(|group| i32::from(group.count) * if group.sign > 0 { i32::from(group.sides) } else { -1 })
        .sum();
    if maximum == 0 { return Ok(formula.into()); }
    Ok(format!("{formula}{}", signed(maximum)))
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

/// The GM's natural 20 is critical on an adversary attack (not on an ordinary damage d20).
pub fn adversary_critical(dice: &[crate::dice::ThrownDie]) -> bool {
    dice.iter().any(|die| die.sides == 20 && die.value == 20 && die.sign > 0)
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
            cast_asset_id: None,
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
            cast_asset_id: None,
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
            cast_asset_id: None,
            };
        assert_eq!(damage_formula(&foe), "flat:2");
    }

    #[test]
    fn critical_damage_adds_maximum_dice_not_flat_modifiers() {
        assert_eq!(critical_damage_formula("2d8+1").unwrap(), "2d8+1+16");
        assert_eq!(critical_damage_formula("1d6+1d8+3").unwrap(), "1d6+1d8+3+14");
        assert_eq!(critical_damage_formula("flat:4").unwrap(), "flat:4");
    }

    #[test]
    fn adversary_natural_twenty_hits_even_above_the_modified_total() {
        let mut die = crate::dice::ThrownDie { sides: 20, value: 20, sign: 1, tag: None, keyframes: Vec::new() };
        assert!(attack_hits(18, 25, adversary_critical(std::slice::from_ref(&die))));
        die.value = 19;
        assert!(!adversary_critical(std::slice::from_ref(&die)));
        die.sides = 12;
        die.value = 12;
        assert!(!adversary_critical(std::slice::from_ref(&die)));
    }

    #[test]
    fn proficiency_scales_bare_weapon_dice_and_direct_damage_is_preserved() {
        let mut sheet = character(CharacterKind::Pc, BTreeMap::from([("proficiency".into(), 3)]), "");
        sheet.texts.insert("damage".into(), "d8+2 direct phy".into());
        assert_eq!(damage_formula(&sheet), "3d8+2");
        assert!(deals_direct_damage(&sheet));
        sheet.texts.insert("damage".into(), "2d8+2 mag".into());
        assert_eq!(damage_formula(&sheet), "2d8+2");
        assert!(!deals_direct_damage(&sheet));
    }
}
