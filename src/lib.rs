#![deny(rust_2018_idioms)]

pub mod ast;
pub mod cache;
pub mod lexer;
pub mod lifetime_check;
pub mod lint;
pub mod llvm_lowering;
mod mock;
pub mod name_resolution;
pub mod parser;
pub mod semantic_check;
pub mod visitor;
