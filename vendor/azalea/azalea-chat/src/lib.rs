#![doc = include_str!("../README.md")]

pub mod base_component;
mod component;
pub mod keybind;
#[cfg(feature = "numbers")]
pub mod numbers;
pub mod style;
pub mod text_component;
pub mod translatable_component;

pub use component::FormattedText;
