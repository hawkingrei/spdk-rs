extern crate spdk_sys;

use spdk_sys::*;

fn main() {
    let opt: *mut spdk_env_opts = &mut Default::default();
    unsafe {
        spdk_env_opts_init(opt);
    }
    println!("Hello, world!");
}

