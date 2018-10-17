extern crate libc;
extern crate spdk_sys;

use spdk_sys::*;
use std::ffi::CString;
use std::process;
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

fn probe_cb(
    cb_ctx: *mut libc::c_void,
    trid: *const spdk_nvme_transport_id,
    opts: *mut spdk_nvme_ctrlr_opts,
) -> bool {
    println!("{:?}", escape(&((*trid).traddr)));
    return true;
}

pub fn escape(data: &[i8; 257]) -> String {
    let mut escaped = Vec::with_capacity(data.len() * 4);
    for c in data.iter() {
        match c as u8 {
            b'\n' => escaped.extend_from_slice(br"\n"),
            b'\r' => escaped.extend_from_slice(br"\r"),
            b'\t' => escaped.extend_from_slice(br"\t"),
            b'"' => escaped.extend_from_slice(b"\\\""),
            b'\\' => escaped.extend_from_slice(br"\\"),
            _ => {
                if c >= 0x20 && c < 0x7f {
                    // c is printable
                    escaped.push(c);
                } else {
                    escaped.push(b'\\');
                    escaped.push(b'0' + (c >> 6));
                    escaped.push(b'0' + ((c >> 3) & 7));
                    escaped.push(b'0' + (c & 7));
                }
            }
        }
    }
    escaped.shrink_to_fit();
    unsafe { String::from_utf8_unchecked(escaped) }
}

fn main() {
    let opt: *mut spdk_env_opts = &mut Default::default();
    unsafe {
        /*
         * SPDK relies on an abstraction around the local environment
         * named env that handles memory allocation and PCI device operations.
         * This library must be initialized first.
         *
         */
        spdk_env_opts_init(opt);
        (*opt).name = CString::new("hello_world")
            .expect("CString::new failed")
            .as_ptr();
        (*opt).shm_id = 0;
        if (spdk_env_init(opt) < 0) {
            println!("Initializing NVMe Controllers\n");
            process::exit(1);
        }
        println!("Initializing NVMe Controllers\n");
        /*
         * Start the SPDK NVMe enumeration process.  probe_cb will be called
         *  for each NVMe controller found, giving our application a choice on
         *  whether to attach to each controller.  attach_cb will then be
         *  called for each controller after the SPDK NVMe driver has completed
         *  initializing the controller we chose to attach.
         */
        //let rc = rc = spdk_nvme_probe(ptr::null(), ptr::null(), probe_cb, attach_cb, ptr::null());
    }
}
