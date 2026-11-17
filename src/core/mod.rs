//! Tina Core - Coordenação central
//!
//! O núcleo da Tina responsável por orquestrar todos os componentes
//! e coordenar as operações entre eles.

mod tina;
mod scheduler;

pub use tina::*;
pub use scheduler::*;