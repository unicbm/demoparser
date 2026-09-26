// Process-wide allocator for the parser's allocation-heavy per-tick workload.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod demo_network_handle;
#[cfg(all(test, feature = "external-demo-tests"))]
pub mod e2e_test;
pub mod first_pass;
pub mod maps;
pub mod parse_demo;
mod profile;
pub mod second_pass;
