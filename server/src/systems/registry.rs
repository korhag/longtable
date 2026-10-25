// ------------------------------------------------------------
// Module: registry
// Purpose: The one place shared code asks for a system's rules.
//          Adding a system with code means one line here.
// ------------------------------------------------------------

use super::daggerheart::Daggerheart;
use super::rules::{Generic, Rules};

pub fn rules_for(id: &str) -> Box<dyn Rules> {
    match id {
        "daggerheart" => Box::new(Daggerheart),
        _ => Box::new(Generic),
    }
}
