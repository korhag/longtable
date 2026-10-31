// ------------------------------------------------------------
// Module: dice
// Purpose: The shared dice. Formulas are parsed here, shapes are
//          defined once, and the server simulates the tumble so
//          every client plays the same frames and the same result.
// ------------------------------------------------------------

mod formula;
mod geometry;
mod simulate;

pub use formula::{parse_formula, RollFormula};
pub use geometry::{templates, DieKind};
pub use simulate::{simulate, ThrownDie, KEYFRAME_DT};
#[cfg(test)]
pub use simulate::SimFrame;
