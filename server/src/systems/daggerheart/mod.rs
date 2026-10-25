// ------------------------------------------------------------
// Module: daggerheart
// Purpose: Hope and Fear. Equal dice are a critical success.
//          Otherwise the higher die says the roll is with Hope
//          or with Fear. This is the only place that knows that.
// ------------------------------------------------------------

use crate::dice::{RollFormula, ThrownDie};

use super::rules::{RollOutcome, Rules};

pub struct Daggerheart;

impl Rules for Daggerheart {
    fn check_roll(&self, formula: &RollFormula) -> Result<(), String> {
        let hope = formula.groups.iter().filter(|group| group.tag.as_deref() == Some("h")).count();
        let fear = formula.groups.iter().filter(|group| group.tag.as_deref() == Some("f")).count();
        if hope == 0 && fear == 0 {
            return Ok(());
        }
        if hope != 1 || fear != 1 {
            return Err("a duality roll needs exactly one Hope die and one Fear die".into());
        }
        for group in &formula.groups {
            if group.tag.as_deref() != Some("h") && group.tag.as_deref() != Some("f") {
                continue;
            }
            if group.count != 1 {
                return Err("Hope and Fear are each a single die".into());
            }
            if group.sign != 1 {
                return Err("Hope and Fear are added to the roll".into());
            }
        }
        Ok(())
    }

    fn outcome(&self, dice: &[ThrownDie], _total: i32) -> Option<RollOutcome> {
        let hope = dice.iter().find(|die| die.tag.as_deref() == Some("h"))?;
        let fear = dice.iter().find(|die| die.tag.as_deref() == Some("f"))?;
        let (label, tone) = if hope.value == fear.value {
            ("Critical success", "critical")
        } else if hope.value > fear.value {
            ("with Hope", "hope")
        } else {
            ("with Fear", "fear")
        };
        Some(RollOutcome { label: label.into(), tone: tone.into() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::{parse_formula, SimFrame};

    fn die(tag: &str, value: u8) -> ThrownDie {
        ThrownDie {
            sides: 12,
            value,
            sign: 1,
            tag: Some(tag.into()),
            keyframes: vec![SimFrame { p: [0.0, 0.0, 0.0], q: [0.0, 0.0, 0.0, 1.0] }],
        }
    }

    #[test]
    fn equal_dice_are_a_critical_success() {
        let result = Daggerheart.outcome(&[die("h", 7), die("f", 7)], 14).unwrap();
        assert_eq!(result.label, "Critical success");
        assert_eq!(result.tone, "critical");
    }

    #[test]
    fn the_higher_die_names_the_roll() {
        assert_eq!(Daggerheart.outcome(&[die("h", 9), die("f", 4)], 13).unwrap().tone, "hope");
        assert_eq!(Daggerheart.outcome(&[die("h", 2), die("f", 8)], 10).unwrap().tone, "fear");
    }

    #[test]
    fn an_ordinary_roll_has_no_outcome() {
        assert!(Daggerheart.outcome(&[die("x", 4)], 4).is_none());
    }

    #[test]
    fn a_duality_roll_needs_the_pair() {
        assert!(Daggerheart.check_roll(&parse_formula("1d12h+1d12f+1d6").unwrap()).is_ok());
        assert!(Daggerheart.check_roll(&parse_formula("1d12h").unwrap()).is_err());
        assert!(Daggerheart.check_roll(&parse_formula("2d12h+1d12f").unwrap()).is_err());
        assert!(Daggerheart.check_roll(&parse_formula("-1d12h+1d12f").unwrap()).is_err());
    }
}
