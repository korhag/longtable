// ------------------------------------------------------------
// Module: dice
// Purpose: The shared dice. Formulas are parsed here, shapes are
//          defined once, and the server simulates the tumble so
//          every client plays the same frames and the same result.
// ------------------------------------------------------------

mod formula;
mod geometry;
mod simulate;

pub use formula::{parse_formula, DiceGroup, FormulaError, RollFormula};
pub use geometry::{read_face, templates, DieKind, DieTemplate, Face};
pub use simulate::{simulate, SimFrame, ThrownDie, KEYFRAME_DT};
