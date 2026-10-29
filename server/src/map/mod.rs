// ------------------------------------------------------------
// Module: map
// Purpose: Scene geometry imported from a battle map, plus the
//          Universal VTT reader used by Dungeon Alchemist.
// ------------------------------------------------------------

pub mod compile;
pub mod drafts;
pub mod layout;
mod package;
pub(crate) mod presentation;
mod scene;
mod uvtt;

#[allow(unused_imports)]
pub use drafts::{Draft, DraftStore};
#[allow(unused_imports)]
pub use layout::MapLayout;
#[allow(unused_imports)]
pub use scene::{Door, Light, Scene, Segment, Vec2, DIMETRIC_PROFILE};
#[allow(unused_imports)]
pub use package::{read_package, safe_relative, LoadedPackage, PackageError};
#[allow(unused_imports)]
pub use presentation::Prerendered;
pub use uvtt::{parse_uvtt, ParsedMap, UvttError};
