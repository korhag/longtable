// ------------------------------------------------------------
// Module: vision
// Purpose: Server-owned line of sight and the party's explored
//          fog. Clients only draw what they are sent.
// ------------------------------------------------------------

mod fog;
mod visibility;

pub use fog::{cells_for, Explored, CELLS_PER_SQUARE};
pub use visibility::{path_blocked, point_in_poly, visibility_polygon};
