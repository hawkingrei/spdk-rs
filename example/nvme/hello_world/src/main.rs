extern crate spdk_sys;

use spdk_sys::*;
use std::ptr;
struct ctrlr_entry {
    ctrlr: *mut spdk_nvme_ctrlr,
    next: *mut ctrlr_entry,
    name: [u8; 1024],
}

unsafe impl Send for ctrlr_entry {}
unsafe impl Sync for ctrlr_entry {}

struct ns_entry {
    ctrlr: *mut spdk_nvme_ctrlr,
    ns: *mut spdk_nvme_ns,
    next: *mut ns_entry,
    qpair: *mut spdk_nvme_qpair,
}

unsafe impl Send for ns_entry {}
unsafe impl Sync for ns_entry {}

static g_controllers: Arc<Option<ctrlr_entry>>;
static g_namespaces: Arc<Option<ns_entry>>;

fn main() {
    let opt: *mut spdk_env_opts = &mut Default::default();
    unsafe {
        spdk_env_opts_init(opt);
    }
    println!("Hello, world!");
}
