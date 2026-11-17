//! Context Manager - seleção e distribuição de contexto

mod distributor;
mod consolidator;
mod selector;

pub use distributor::*;
pub use consolidator::*;
pub use selector::*;