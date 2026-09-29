//! TOML configuration under XDG `~/.config/nixpresence/config.toml`.
mod defaults;
pub mod load;
mod schema;

#[allow(unused_imports)]
pub use defaults::default_config_toml;
#[allow(unused_imports)]
pub use load::{load_config, load_or_default, validate_config, write_default_config};
pub use schema::*;
