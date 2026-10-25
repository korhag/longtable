// ------------------------------------------------------------
// Module: session
// Purpose: The live table. Rules for who may do what sit next
//          to the state they protect.
// ------------------------------------------------------------

pub(crate) mod permissions;
pub(crate) mod state;

#[allow(unused_imports)]
pub use state::{Seat, Table, Viewer};
