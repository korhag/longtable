// ------------------------------------------------------------
// Module: formula
// Purpose: Parse table shorthand such as `2d6+3` or `d20-1`
//          into dice groups and a flat modifier. A leading
//          minus on a group subtracts that die from the total.
// ------------------------------------------------------------

use thiserror::Error;

use super::geometry::DieKind;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FormulaError {
    #[error("enter a roll, for example 1d20+5")]
    Empty,
    #[error("that formula is too long")]
    TooLong,
    #[error("could not read '{0}'")]
    Bad(String),
    #[error("d{0} is not on this table (use d4, d6, d8, d10, d12, or d20)")]
    Sides(u32),
    #[error("roll at most 20 dice at once")]
    TooMany,
    #[error("that roll has no dice")]
    NoDice,
}

/// One cluster of identical dice, with the sign applied to each result.
/// `tag` is a system's die letter, such as `h`, when the formula asked for one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiceGroup {
    pub count: u8,
    pub sides: u8,
    pub sign: i8,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollFormula {
    pub groups: Vec<DiceGroup>,
    pub modifier: i32,
}

impl RollFormula {
    /// Expand groups into the dice that will actually be thrown, in order.
    pub fn dice(&self) -> Vec<(DieKind, i8, Option<String>)> {
        let mut dice = Vec::new();
        for group in &self.groups {
            let kind = DieKind::from_sides(u32::from(group.sides)).expect("parser only emits known dice");
            for _ in 0..group.count {
                dice.push((kind, group.sign, group.tag.clone()));
            }
        }
        dice
    }
}

pub fn parse_formula(input: &str) -> Result<RollFormula, FormulaError> {
    let compact: String = input
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .flat_map(|ch| ch.to_lowercase())
        .collect();
    if compact.is_empty() {
        return Err(FormulaError::Empty);
    }
    if compact.len() > 48 {
        return Err(FormulaError::TooLong);
    }

    let bytes = compact.as_bytes();
    let mut index = 0;
    let mut groups = Vec::new();
    let mut modifier = 0i32;
    let mut total_dice = 0u32;
    let mut first = true;

    while index < bytes.len() {
        let mut sign = 1i32;
        if bytes[index] == b'+' {
            index += 1;
        } else if bytes[index] == b'-' {
            sign = -1;
            index += 1;
        } else if !first {
            return Err(FormulaError::Bad(compact));
        }
        first = false;
        if index >= bytes.len() {
            return Err(FormulaError::Bad(compact));
        }

        let number_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        let number = &compact[number_start..index];

        if index < bytes.len() && bytes[index] == b'd' {
            let count: u32 = if number.is_empty() {
                1
            } else {
                number.parse().map_err(|_| FormulaError::Bad(compact.clone()))?
            };
            index += 1;
            let sides_start = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            if sides_start == index {
                return Err(FormulaError::Bad(compact));
            }
            let sides: u32 = compact[sides_start..index]
                .parse()
                .map_err(|_| FormulaError::Bad(compact.clone()))?;
            let kind = DieKind::from_sides(sides).ok_or(FormulaError::Sides(sides))?;
            let tag = if index < bytes.len() && bytes[index].is_ascii_lowercase() {
                let letter = compact[index..=index].to_string();
                index += 1;
                Some(letter)
            } else {
                None
            };
            if count == 0 || count > 20 {
                return Err(FormulaError::TooMany);
            }
            total_dice += count;
            if total_dice > 20 {
                return Err(FormulaError::TooMany);
            }
            groups.push(DiceGroup {
                count: count as u8,
                sides: kind.sides(),
                sign: sign as i8,
                tag,
            });
        } else {
            if number.is_empty() {
                return Err(FormulaError::Bad(compact));
            }
            let value: i32 = number.parse().map_err(|_| FormulaError::Bad(compact.clone()))?;
            modifier += sign * value;
        }
    }

    if groups.is_empty() {
        return Err(FormulaError::NoDice);
    }
    if !(-1000..=1000).contains(&modifier) {
        return Err(FormulaError::Bad(compact));
    }
    Ok(RollFormula { groups, modifier })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_rolls() {
        let roll = parse_formula("d20").unwrap();
        assert_eq!(roll.dice().len(), 1);
        assert_eq!(roll.modifier, 0);

        let roll = parse_formula("2d6 + 3").unwrap();
        assert_eq!(roll.groups.len(), 1);
        assert_eq!(roll.groups[0].count, 2);
        assert_eq!(roll.groups[0].sides, 6);
        assert_eq!(roll.modifier, 3);

        let roll = parse_formula("2d6+1d8-1").unwrap();
        assert_eq!(roll.groups.len(), 2);
        assert_eq!(roll.groups[1].sides, 8);
        assert_eq!(roll.modifier, -1);

        let roll = parse_formula("2d6-1d4").unwrap();
        assert_eq!(roll.groups[1].sign, -1);
    }

    #[test]
    fn rejects_dice_this_table_does_not_carry() {
        let roll = parse_formula("1d12h+1d12f+1d6").unwrap();
        assert_eq!(roll.groups[0].tag.as_deref(), Some("h"));
        assert_eq!(roll.groups[1].tag.as_deref(), Some("f"));
        assert_eq!(roll.groups[2].tag, None);
        assert_eq!(roll.dice().len(), 3);

        let roll = parse_formula("1d20h+1d12f-2").unwrap();
        assert_eq!(roll.groups[0].sides, 20);
        assert_eq!(roll.modifier, -2);
        assert!(parse_formula("1d12h").is_ok());

        assert!(matches!(parse_formula("1d7"), Err(FormulaError::Sides(7))));
        assert!(matches!(parse_formula("99d6"), Err(FormulaError::TooMany)));
        assert!(matches!(parse_formula(""), Err(FormulaError::Empty)));
        assert!(matches!(parse_formula("3"), Err(FormulaError::NoDice)));
        assert!(parse_formula("nope").is_err());
    }
}
