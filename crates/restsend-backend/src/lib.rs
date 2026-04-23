extern crate self as restsend_domain;

pub mod api;
pub mod app;
pub mod entity;
pub mod infra;
pub mod model;
pub mod openapi;
pub mod services;

pub use model::*;
pub use openapi::*;
