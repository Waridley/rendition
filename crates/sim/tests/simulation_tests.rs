//! This file is necessary to pull in submodules. Normally integration tests are a flat directory,
//! but we have a more complex hierarchy. Using the `module_name.rs` + `module_name/...` doesn't
//! work in the tests directory, so we have to use `module_name/mod.rs` and import those modules in
//! a top-level file.

mod determinism;
mod robustness;
