#![warn(rust_2018_idioms)]
#![feature(async_await, await_macro, futures_api)]
#![feature(tool_lints)]
#![feature(nll)]
#![allow(macro_use_extern_crate)]
#![allow(warnings)]
#![allow(clippy)]
#![allow(unknown_lints)]
include!("./spdk_bindings.rs");

pub const SPDK_MALLOC_DMA: u32 = 1;
pub const SPDK_ENV_SOCKET_ID_ANY: i32 = -1;
