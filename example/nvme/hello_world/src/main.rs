extern crate spdk_rs;

use spdk_rs::env;

fn main() {
    println!("Hello, world!");
    let opt: *mut env::spdk_env_opts = &mut Default::default();
    unsafe {
        env::spdk_env_opts_init(opt);
    }
    println!("Hello, world!");
}

