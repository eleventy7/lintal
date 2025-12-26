//! Indentation handlers for different Java constructs.
//!
//! Each handler is responsible for checking indentation of a specific
//! type of Java construct (class, method, if statement, etc.)

mod base;

pub use base::{HandlerContext, IndentHandler};
