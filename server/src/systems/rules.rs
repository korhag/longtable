// ------------------------------------------------------------
// Module: rules
// Purpose: The small hook a system uses when a roll needs more
//          than the shared dice. A system with no code uses
//          Generic, which accepts the roll and adds no outcome.
// ------------------------------------------------------------

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::dice::{RollFormula, ThrownDie};

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct RollOutcome {
    pub label: String,
    pub tone: String,
}

pub trait Rules: Send + Sync {
    fn check_roll(&self, _formula: &RollFormula) -> Result<(), String> {
        Ok(())
    }

    fn outcome(&self, _dice: &[ThrownDie], _total: i32) -> Option<RollOutcome> {
        None
    }
}

pub struct Generic;

impl Rules for Generic {}
