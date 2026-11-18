//! Tina Core - Coordenação central
//!
//! O núcleo da Tina responsável por orquestrar todos os componentes
//! e coordenar as operações entre eles.

pub mod company;
pub mod department;
pub mod installation;
pub mod position;
mod scheduler;
mod tina;

pub use company::*;
pub use department::*;
pub use installation::*;
pub use position::*;
pub use scheduler::*;
pub use tina::*;
