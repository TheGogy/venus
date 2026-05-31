#[allow(clippy::all, clippy::pedantic, dead_code, non_upper_case_globals, non_snake_case, non_camel_case_types)]
#[cfg(feature = "syzygy")]
mod binds;

#[cfg(feature = "syzygy")]
mod attack_if;

pub mod probe;
