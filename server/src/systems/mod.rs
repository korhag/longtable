// ------------------------------------------------------------
// Module: systems
// Purpose: The shared description of a game system. Data lives
//          in systems/<id>/system.json. Rules that data cannot
//          express live in this module's matching folder.
// ------------------------------------------------------------

mod def;
mod fill;
mod load;
pub mod daggerheart;
mod registry;
mod rules;

pub use def::{
    field_ceiling, system_id_ok, Campaign, Character, CharacterKind, FieldKind, SystemDef,
};
pub use fill::{apply_modifiers, fill_formula};
pub use load::{load_systems, save_system, SystemCatalog};
pub use registry::rules_for;
pub use rules::{RollOutcome, Rules};
