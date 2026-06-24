#![warn(clippy::all)]
#![allow(clippy::module_inception)]

pub mod app_state;
pub mod config;
pub mod controllers;
pub mod docs;
pub mod errors;
pub mod infrastructure;
pub mod middleware;
pub mod models;
pub mod policies;

pub mod extractors;
pub mod serializers;
pub mod services;
pub mod utils;
