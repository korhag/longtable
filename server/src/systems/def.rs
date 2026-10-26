// ------------------------------------------------------------
// Module: def
// Purpose: The data a system is made of, and the characters
//          that play under one. `cargo test` writes the
//          matching TypeScript into the client.
// ------------------------------------------------------------

use std::collections::{BTreeMap, HashSet};

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
    /// One designed sheet for each kind of character.
    #[serde(default)]
    pub sheets: CharacterSheets,
    #[serde(default)]
    pub grid: Option<GridRule>,
    /// Older files used one sheet for every kind. Kept only so those files still open.
    #[serde(default, skip_serializing)]
    #[ts(skip)]
    pub sheet: SheetLayout,
    /// Older files used one toolbar for every kind.
    #[serde(default, skip_serializing)]
    #[ts(skip)]
    pub toolbar: ToolbarLayout,
}

impl SystemDef {
    pub fn sheet_for(&self, kind: CharacterKind) -> &KindSheet {
        match kind {
            CharacterKind::Pc => &self.sheets.pc,
            CharacterKind::Npc => &self.sheets.npc,
            CharacterKind::Adversary => &self.sheets.adversary,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct CharacterSheets {
    #[serde(default)]
    pub pc: KindSheet,
    #[serde(default)]
    pub npc: KindSheet,
    #[serde(default)]
    pub adversary: KindSheet,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct KindSheet {
    #[serde(default)]
    pub sections: Vec<SheetSection>,
    #[serde(default)]
    pub toolbar: ToolbarLayout,
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
    /// How a pool is drawn: boxes, pips, or a plain count.
    #[serde(default)]
    pub look: PoolLook,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub steps: Vec<ThresholdStep>,
    /// The pool a threshold marks when damage is applied.
    #[serde(default)]
    pub pool: Option<String>,
    /// The starting formula of a dice thing.
    #[serde(default)]
    pub formula: String,
    #[serde(default)]
    pub hint: String,
}

impl Default for FieldDef {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            kind: FieldKind::Number,
            min: 0,
            max: None,
            max_from: None,
            default: 0,
            text_default: String::new(),
            look: PoolLook::Boxes,
            options: Vec::new(),
            steps: Vec::new(),
            pool: None,
            formula: String::new(),
            hint: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum FieldKind {
    Number,
    Text,
    Choice,
    Pool,
    Thresholds,
    Dice,
}

impl<'de> Deserialize<'de> for FieldKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        match raw.as_str() {
            "number" => Ok(FieldKind::Number),
            "text" => Ok(FieldKind::Text),
            "choice" => Ok(FieldKind::Choice),
            "pool" | "track" | "pips" => Ok(FieldKind::Pool),
            "thresholds" => Ok(FieldKind::Thresholds),
            "dice" => Ok(FieldKind::Dice),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["number", "text", "choice", "pool", "track", "pips", "thresholds", "dice"],
            )),
        }
    }
}

impl FieldKind {
    pub fn holds_number(self) -> bool {
        matches!(self, FieldKind::Number | FieldKind::Pool)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum PoolLook {
    #[default]
    Boxes,
    Pips,
    Count,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ThresholdStep {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub default: i32,
    #[serde(default = "one_mark")]
    pub marks: i32,
}

fn one_mark() -> i32 {
    1
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

/// Copy a file that still has one shared sheet onto PC, NPC, and adversary.
pub fn adopt_legacy(system: &mut SystemDef) {
    let legacy = !system.sheet.sections.is_empty() || !system.toolbar.resources.is_empty() || !system.toolbar.actions.is_empty();
    if !legacy || !sheets_blank(&system.sheets) {
        return;
    }
    let kind = KindSheet {
        sections: system.sheet.sections.clone(),
        toolbar: system.toolbar.clone(),
    };
    system.sheets.pc = kind.clone();
    system.sheets.npc = kind.clone();
    system.sheets.adversary = kind;
}

fn sheets_blank(sheets: &CharacterSheets) -> bool {
    [&sheets.pc, &sheets.npc, &sheets.adversary].into_iter().all(|sheet| {
        sheet.sections.is_empty() && sheet.toolbar.resources.is_empty() && sheet.toolbar.actions.is_empty()
    })
}

pub fn validate(system: &SystemDef) -> Result<(), String> {
    if !system_id_ok(&system.id) {
        return Err(format!("'{}' is not a system id (use 2 to 32 lowercase letters or digits)", system.id));
    }
    if system.name.trim().is_empty() {
        return Err("a system needs a name".into());
    }
    let mut field_ids = HashSet::new();
    for field in &system.fields {
        if !field_id_ok(&field.id) {
            return Err(format!("'{}' is not a field id", field.id));
        }
        if !field_ids.insert(field.id.clone()) {
            return Err(format!("the thing '{}' is listed twice", field.id));
        }
        if field.name.trim().is_empty() {
            return Err(format!("the thing '{}' needs a name", field.id));
        }
    }
    for field in &system.fields {
        if let Some(source) = &field.max_from {
            match system.fields.iter().find(|item| &item.id == source) {
                Some(source) if source.kind.holds_number() => {}
                Some(_) => return Err(format!("'{}' takes its maximum from a thing that is not a number", field.id)),
                None => return Err(format!("'{}' takes its maximum from '{}', which is not a thing", field.id, source)),
            }
        }
        if field.kind == FieldKind::Choice && !field.options.iter().any(|option| !option.trim().is_empty()) {
            return Err(format!("the choice '{}' needs an option", field.name));
        }
    }
    let mut step_ids = HashSet::new();
    for field in &system.fields {
        if field.kind != FieldKind::Thresholds {
            continue;
        }
        match field.pool.as_deref() {
            Some(pool) => match system.fields.iter().find(|item| item.id == pool) {
                Some(item) if item.kind == FieldKind::Pool => {}
                Some(item) => return Err(format!("'{}' marks '{}', which is not a pool", field.name, item.id)),
                None => return Err(format!("'{}' marks '{}', which is not a thing", field.name, pool)),
            },
            None => return Err(format!("'{}' needs a pool to mark", field.name)),
        }
        if field.steps.is_empty() {
            return Err(format!("'{}' needs a step, such as Minor", field.name));
        }
        for step in &field.steps {
            if !field_id_ok(&step.id) {
                return Err(format!("'{}' is not a step id", step.id));
            }
            if field_ids.contains(&step.id) {
                return Err(format!("the step '{}' uses the same id as a thing", step.id));
            }
            if !step_ids.insert(step.id.clone()) {
                return Err(format!("the step '{}' is listed twice", step.id));
            }
            if step.name.trim().is_empty() {
                return Err(format!("the step '{}' needs a name", step.id));
            }
        }
    }
    let mut action_ids = HashSet::new();
    for action in &system.actions {
        if !field_id_ok(&action.id) {
            return Err(format!("'{}' is not an action id", action.id));
        }
        if !action_ids.insert(action.id.clone()) {
            return Err(format!("the roll '{}' is listed twice", action.id));
        }
        if system.fields.iter().any(|field| field.id == action.id && field.kind == FieldKind::Dice) {
            return Err(format!("the roll '{}' uses the same id as a dice thing", action.id));
        }
        if action.name.trim().is_empty() {
            return Err(format!("the roll '{}' needs a name", action.id));
        }
        check_formula(system, &format!("the roll '{}'", action.name), &action.formula)?;
    }
    for field in &system.fields {
        if field.kind == FieldKind::Dice {
            check_formula(system, &format!("the thing '{}'", field.name), &field.formula)?;
        }
    }
    let mut letters = HashSet::new();
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
    check_kind_sheet(system, &system.sheets.pc, "PC", &field_ids, &action_ids)?;
    check_kind_sheet(system, &system.sheets.npc, "NPC", &field_ids, &action_ids)?;
    check_kind_sheet(system, &system.sheets.adversary, "adversary", &field_ids, &action_ids)?;
    Ok(())
}

fn check_kind_sheet(
    system: &SystemDef,
    sheet: &KindSheet,
    label: &str,
    field_ids: &HashSet<String>,
    action_ids: &HashSet<String>,
) -> Result<(), String> {
    let mut seen = HashSet::new();
    for section in &sheet.sections {
        if !(1..=4).contains(&section.columns) {
            return Err(format!("the {label} section '{}' needs 1 to 4 columns", section.title));
        }
        for id in &section.fields {
            if !field_ids.contains(id) {
                return Err(format!("the {label} sheet lists '{id}', which is not a thing"));
            }
            if !seen.insert(id.clone()) {
                return Err(format!("the {label} sheet lists '{id}' twice"));
            }
        }
    }
    for id in &sheet.toolbar.resources {
        match system.fields.iter().find(|field| &field.id == id) {
            Some(field) if field.kind.holds_number() => {}
            Some(_) => return Err(format!("the {label} toolbar lists '{id}', which is not a number or a pool")),
            None => return Err(format!("the {label} toolbar lists '{id}', which is not a thing")),
        }
    }
    for id in &sheet.toolbar.actions {
        let dice = system.fields.iter().any(|field| &field.id == id && field.kind == FieldKind::Dice);
        if !action_ids.contains(id) && !dice {
            return Err(format!("the {label} toolbar lists '{id}', which is not a roll"));
        }
    }
    Ok(())
}

fn check_formula(system: &SystemDef, label: &str, formula: &str) -> Result<(), String> {
    for reference in references(formula) {
        if !number_source(system, &reference) {
            return Err(format!("{label} reads '{reference}', which is not a number"));
        }
    }
    Ok(())
}

fn number_source(system: &SystemDef, id: &str) -> bool {
    if let Some(field) = system.fields.iter().find(|field| field.id == id) {
        return field.kind.holds_number();
    }
    system
        .fields
        .iter()
        .filter(|field| field.kind == FieldKind::Thresholds)
        .any(|field| field.steps.iter().any(|step| step.id == id))
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
        if let Some(value) = numbers.get(source).copied() {
            return value.max(0);
        }
    }
    field.max.unwrap_or(1000).max(0)
}

/// The marks of the highest step whose value is at most `amount`.
/// Nothing is marked when the amount sits below every step.
pub fn threshold_marks(field: &FieldDef, amount: i32, numbers: &BTreeMap<String, i32>) -> Option<i32> {
    let mut best: Option<(&ThresholdStep, i32)> = None;
    for step in &field.steps {
        let value = numbers.get(&step.id).copied().unwrap_or(step.default);
        if value <= amount {
            let replace = match best {
                None => true,
                Some((_, best_value)) => value >= best_value,
            };
            if replace {
                best = Some((step, value));
            }
        }
    }
    best.map(|(step, _)| step.marks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SystemDef {
        let hp = FieldDef {
            id: "hp".into(),
            name: "HP".into(),
            kind: FieldKind::Pool,
            max: Some(6),
            ..FieldDef::default()
        };
        let strength = FieldDef {
            id: "str".into(),
            name: "Strength".into(),
            kind: FieldKind::Number,
            min: -5,
            max: Some(6),
            ..FieldDef::default()
        };
        let sheet = KindSheet {
            sections: vec![SheetSection { title: "Stats".into(), columns: 2, fields: vec!["str".into(), "hp".into()] }],
            toolbar: ToolbarLayout { resources: vec!["hp".into()], actions: vec!["strike".into()] },
        };
        SystemDef {
            id: "sample".into(),
            name: "Sample".into(),
            fields: vec![strength, hp],
            die_tags: Vec::new(),
            actions: vec![ActionDef { id: "strike".into(), name: "Strike".into(), formula: "1d20+@str".into() }],
            sheets: CharacterSheets { pc: sheet, ..CharacterSheets::default() },
            grid: None,
            sheet: SheetLayout::default(),
            toolbar: ToolbarLayout::default(),
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

    #[test]
    fn a_threshold_must_mark_a_pool_and_keep_unique_steps() {
        let mut system = sample();
        system.fields.push(FieldDef {
            id: "thresholds".into(),
            name: "Thresholds".into(),
            kind: FieldKind::Thresholds,
            pool: Some("str".into()),
            steps: vec![ThresholdStep { id: "minor".into(), name: "Minor".into(), default: 1, marks: 1 }],
            ..FieldDef::default()
        });
        let err = validate(&system).unwrap_err();
        assert!(err.contains("not a pool"), "{err}");

        system.fields.last_mut().unwrap().pool = Some("hp".into());
        system.fields.last_mut().unwrap().steps.push(ThresholdStep {
            id: "hp".into(),
            name: "Major".into(),
            default: 7,
            marks: 2,
        });
        let err = validate(&system).unwrap_err();
        assert!(err.contains("same id as a thing"), "{err}");

        system.fields.last_mut().unwrap().steps[1].id = "major".into();
        validate(&system).unwrap();
    }

    #[test]
    fn a_toolbar_roll_can_be_a_dice_thing_and_a_choice_needs_an_option() {
        let mut system = sample();
        system.fields.push(FieldDef {
            id: "kind".into(),
            name: "Type".into(),
            kind: FieldKind::Choice,
            ..FieldDef::default()
        });
        let err = validate(&system).unwrap_err();
        assert!(err.contains("option"), "{err}");
        system.fields.last_mut().unwrap().options = vec!["Bruiser".into()];

        system.fields.push(FieldDef {
            id: "damage".into(),
            name: "Damage".into(),
            kind: FieldKind::Dice,
            formula: "1d8+@str".into(),
            ..FieldDef::default()
        });
        system.sheets.pc.toolbar.actions.push("damage".into());
        validate(&system).unwrap();

        system.sheets.pc.toolbar.actions.push("missing".into());
        let err = validate(&system).unwrap_err();
        assert!(err.contains("missing"), "{err}");
    }

    #[test]
    fn a_missing_maximum_source_falls_back_to_the_fixed_maximum() {
        let field = FieldDef {
            id: "hp".into(),
            name: "HP".into(),
            kind: FieldKind::Pool,
            max: Some(6),
            max_from: Some("hpmax".into()),
            ..FieldDef::default()
        };
        assert_eq!(field_ceiling(&field, &BTreeMap::new()), 6);
        let mut numbers = BTreeMap::new();
        numbers.insert("hpmax".into(), 9);
        assert_eq!(field_ceiling(&field, &numbers), 9);
    }

    #[test]
    fn thresholds_pick_the_highest_step_at_or_below_the_amount() {
        let field = FieldDef {
            id: "thresholds".into(),
            name: "Thresholds".into(),
            kind: FieldKind::Thresholds,
            steps: vec![
                ThresholdStep { id: "minor".into(), name: "Minor".into(), default: 1, marks: 1 },
                ThresholdStep { id: "major".into(), name: "Major".into(), default: 7, marks: 2 },
                ThresholdStep { id: "severe".into(), name: "Severe".into(), default: 13, marks: 3 },
            ],
            pool: Some("hp".into()),
            ..FieldDef::default()
        };
        assert_eq!(threshold_marks(&field, 0, &BTreeMap::new()), None);
        assert_eq!(threshold_marks(&field, 1, &BTreeMap::new()), Some(1));
        assert_eq!(threshold_marks(&field, 7, &BTreeMap::new()), Some(2));
        assert_eq!(threshold_marks(&field, 20, &BTreeMap::new()), Some(3));
    }
}
