// ------------------------------------------------------------
// Module: map
// Purpose: Scene geometry imported from a battle map, plus the
//          Universal VTT reader used by Dungeon Alchemist.
// ------------------------------------------------------------

pub mod compile;
pub mod drafts;
pub mod layout;
mod scene;
mod uvtt;

#[allow(unused_imports)]
pub use drafts::{Draft, DraftStore};
#[allow(unused_imports)]
pub use layout::MapLayout;
#[allow(unused_imports)]
pub use scene::{Door, Light, Scene, Segment, Vec2};
#[allow(unused_imports)]
pub use uvtt::{parse_uvtt, ParsedMap, UvttError};
