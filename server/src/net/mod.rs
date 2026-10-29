// ------------------------------------------------------------
// Module: net
// Purpose: The websocket table and the small HTTP API. The
//          protocol module is the source of the TypeScript types.
// ------------------------------------------------------------

mod accounts;
pub(crate) mod protocol;
mod ws;

#[allow(unused_imports)]
pub use protocol::ServerMsg;
pub use ws::router;
