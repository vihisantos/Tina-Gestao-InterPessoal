//! Tina Core - Coordenação central
//!
//! O núcleo da Tina responsável por orquestrar todos os componentes
//! e coordenar as operações entre eles.

mod tina;
mod scheduler;
pub mod installation;
pub mod company;
pub mod department;
pub mod position;

pub use tina::*;
pub use scheduler::*;
pub use installation::*;
pub use company::*;
pub use department::*;
pub use position::*;