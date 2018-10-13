extern crate spdk_sys;

use spdk_sys::generated;

fn main() {
    let opt: *mut generated::spdk_env_opts = &mut Default::default();
    unsafe {
        generated::spdk_env_opts_init(opt);
    }
    println!("Hello, world!");
}

