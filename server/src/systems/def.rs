// ------------------------------------------------------------
// Module: def
// Purpose: The data a system is made of, and the characters
//          that play under one. `cargo test` writes the
//          matching TypeScript into the client.
// ------------------------------------------------------------

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SystemDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub die_tags: Vec<DieTag>,
    #[serde(default)]
    pub actions: Vec<ActionDef>,
    #[serde(default)]
    pub sheet: SheetLayout,
    #[serde(default)]
    pub toolbar: ToolbarLayout,
    #[serde(default)]
    pub grid: Option<GridRule>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FieldDef {
    pub id: String,
    pub name: String,
    pub kind: FieldKind,
    #[serde(default)]
    pub min: i32,
    #[serde(default)]
    pub max: Option<i32>,
    #[serde(default)]
    pub max_from: Option<String>,
    #[serde(default)]
    pub default: i32,
    #[serde(default)]
    pub text_default: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum FieldKind {
    Number,
    Text,
    Track,
    Pips,
}

impl FieldKind {
    pub fn holds_number(self) -> bool {
        !matches!(self, FieldKind::Text)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct DieTag {
    pub letter: String,
    pub id: String,
    pub name: String,
    pub body: String,
    pub ink: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ActionDef {
    pub id: String,
    pub name: String,
    pub formula: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SheetLayout {
    #[serde(default)]
    pub sections: Vec<SheetSection>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SheetSection {
    pub title: String,
    pub columns: u8,
    pub fields: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ToolbarLayout {
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub actions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct GridRule {
    pub per_square: f32,
    pub unit: String,
    pub diagonal: Diagonal,
    #[serde(default)]
    pub ranges: Vec<RangeBand>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum Diagonal {
    Square,
    Alternate,
    Euclidean,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct RangeBand {
    pub name: String,
    pub up_to: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Campaign {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub system_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum CharacterKind {
    Pc,
    Npc,
    Adversary,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Character {
    pub id: String,
    pub name: String,
    pub kind: CharacterKind,
    #[serde(default)]
    pub owner_id: Option<String>,
    #[serde(default)]
    pub system_id: Option<String>,
    #[serde(default)]
    pub numbers: BTreeMap<String, i32>,
    #[serde(default)]
    pub texts: BTreeMap<String, String>,
}

pub fn validate(system: &SystemDef) -> Result<(), String> {
    if !system_id_ok(&system.id) {
        return Err(format!("'{}' is not a system id (use 2 to 32 lowercase letters or digits)", system.id));
    }
    if system.name.trim().is_empty() {
        return Err("a system needs a name".into());
    }
    let mut field_ids = std::collections::HashSet::new();
    for field in &system.fields {
        if !field_id_ok(&field.id) {
            return Err(format!("'{}' is not a field id", field.id));
        }
        if !field_ids.insert(field.id.clone()) {
            return Err(format!("the field '{}' is listed twice", field.id));
        }
        if field.name.trim().is_empty() {
            return Err(format!("the field '{}' needs a name", field.id));
        }
    }
    for field in &system.fields {
        if let Some(source) = &field.max_from {
            let source = system.fields.iter().find(|item| &item.id == source);
            match source {
                Some(source) if source.kind.holds_number() => {}
                Some(_) => return Err(format!("'{}' takes its maximum from a text field", field.id)),
                None => return Err(format!("'{}' takes its maximum from '{}', which is not a field", field.id, field.max_from.as_deref().unwrap_or(""))),
            }
        }
    }
    let mut letters = std::collections::HashSet::new();
    for tag in &system.die_tags {
        if tag.letter.len() != 1 || !tag.letter.chars().next().is_some_and(|ch| ch.is_ascii_lowercase()) {
            return Err(format!("the die tag '{}' needs a single lowercase letter", tag.name));
        }
        if !letters.insert(tag.letter.clone()) {
            return Err(format!("the letter '{}' is used by two dice", tag.letter));
        }
        if tag.name.trim().is_empty() {
            return Err(format!("the die '{}' needs a name", tag.letter));
        }
    }
    let mut action_ids = std::collections::HashSet::new();
    for action in &system.actions {
        if !field_id_ok(&action.id) {
            return Err(format!("'{}' is not an action id", action.id));
        }
        if !action_ids.insert(action.id.clone()) {
            return Err(format!("the action '{}' is listed twice", action.id));
        }
        for reference in references(&action.formula) {
            let field = system.fields.iter().find(|field| field.id == reference);
            match field {
                Some(field) if field.kind.holds_number() => {}
                Some(_) => return Err(format!("the action '{}' reads '{}', which is text", action.name, reference)),
                None => return Err(format!("the action '{}' reads '{}', which is not a field", action.name, reference)),
            }
        }
    }
    for section in &system.sheet.sections {
        if !(1..=4).contains(&section.columns) {
            return Err(format!("the section '{}' needs 1 to 4 columns", section.title));
        }
        for id in &section.fields {
            if !field_ids.contains(id) {
                return Err(format!("the sheet lists '{}', which is not a field", id));
            }
        }
    }
    for id in &system.toolbar.resources {
        if !field_ids.contains(id) {
            return Err(format!("the toolbar lists '{}', which is not a field", id));
        }
    }
    for id in &system.toolbar.actions {
        if !action_ids.contains(id) {
            return Err(format!("the toolbar lists '{}', which is not an action", id));
        }
    }
    Ok(())
}

pub fn system_id_ok(id: &str) -> bool {
    let len = id.len();
    (2..=32).contains(&len) && id.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}

fn field_id_ok(id: &str) -> bool {
    let mut chars = id.chars();
    let Some(first) = chars.next() else { return false };
    first.is_ascii_lowercase() && id.len() <= 24 && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
}

pub fn references(formula: &str) -> Vec<String> {
    let chars: Vec<char> = formula.chars().collect();
    let mut found = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '@' {
            let start = index + 1;
            let mut end = start;
            while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            if end > start {
                found.push(chars[start..end].iter().collect());
            }
            index = end;
        } else {
            index += 1;
        }
    }
    found
}

pub fn field_ceiling(field: &FieldDef, numbers: &BTreeMap<String, i32>) -> i32 {
    if let Some(source) = &field.max_from {
        return numbers.get(source).copied().unwrap_or(0).max(0);
    }
    field.max.unwrap_or(1000).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SystemDef {
        SystemDef {
            id: "sample".into(),
            name: "Sample".into(),
            fields: vec![FieldDef {
                id: "str".into(),
                name: "Strength".into(),
                kind: FieldKind::Number,
                min: -5,
                max: Some(6),
                max_from: None,
                default: 0,
                text_default: String::new(),
            }],
            die_tags: Vec::new(),
            actions: vec![ActionDef { id: "strike".into(), name: "Strike".into(), formula: "1d20+@str".into() }],
            sheet: SheetLayout { sections: vec![SheetSection { title: "Stats".into(), columns: 2, fields: vec!["str".into()] }] },
            toolbar: ToolbarLayout { resources: vec!["str".into()], actions: vec!["strike".into()] },
            grid: None,
        }
    }

    #[test]
    fn an_action_that_reads_a_missing_field_names_it() {
        let mut system = sample();
        system.actions[0].formula = "1d20+@luck".into();
        let err = validate(&system).unwrap_err();
        assert!(err.contains("luck"), "{err}");
    }

    #[test]
    fn the_folder_id_must_be_plain_lowercase() {
        let mut system = sample();
        system.id = "Pathfinder".into();
        assert!(validate(&system).is_err());
    }
}
