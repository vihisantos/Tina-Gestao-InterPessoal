//! Tina Core - Coordenação central
//!
//! O núcleo da Tina responsável por orquestrar todos os componentes
//! e coordenar as operações entre eles.

pub mod assignment;
pub mod company;
pub mod department;
pub mod employee;
pub mod hierarchy;
pub mod installation;
pub mod position;
mod scheduler;
mod tina;

pub use assignment::*;
pub use company::*;
pub use department::*;
pub use employee::*;
pub use hierarchy::*;
pub use installation::*;
pub use position::*;
pub use scheduler::*;
pub use tina::*;
