#![feature(maybe_uninit)]
extern crate libc;
extern crate spdk_sys;
#[macro_use]
extern crate lazy_static;

use spdk_sys::*;
use std::cell::Cell;
use std::ffi::CString;
use std::mem;
use std::process;
use std::ptr;
use std::sync::Mutex;

#[repr(C)]
struct ctrlr_entry {
    ctrlr: *mut spdk_nvme_ctrlr,
    next: *mut ctrlr_entry,
    name: [u8; 1024],
}

unsafe impl Send for ctrlr_entry {}
unsafe impl Sync for ctrlr_entry {}

#[repr(C)]
struct ns_entry {
    ctrlr: *mut spdk_nvme_ctrlr,
    ns: *mut spdk_nvme_ns,
    next: *mut ns_entry,
    qpair: *mut spdk_nvme_qpair,
}

struct gctlr {
    ctrlr: Cell<*mut ctrlr_entry>,
}

struct gns {
    g_namespaces: Cell<*mut ns_entry>,
}

unsafe impl Send for gns {}
unsafe impl Sync for gns {}

unsafe impl Send for gctlr {}
unsafe impl Sync for gctlr {}

lazy_static! {
    static ref g_controllers: gctlr = gctlr {
        ctrlr: Cell::new(ptr::null_mut()),
    };
    static ref g_namespaces: gns = gns {
        g_namespaces: Cell::new(ptr::null_mut()),
    };
}

unsafe impl Send for ns_entry {}
unsafe impl Sync for ns_entry {}

struct hello_world_sequence {
    buf: *mut libc::c_void,
    next: *mut ns_entry,
    using_cmb_io: u8,
    is_completed: u8,
}

unsafe extern "C" fn probe_cb(
    cb_ctx: *mut libc::c_void,
    trid: *const spdk_nvme_transport_id,
    opts: *mut spdk_nvme_ctrlr_opts,
) -> bool {
    println!("{:?}", escape(&(*trid).traddr));
    return true;
}

unsafe extern "C" fn attach_cb(
    cb_ctx: *mut libc::c_void,
    trid: *const spdk_nvme_transport_id,
    ctrlr: *mut spdk_nvme_ctrlr,
    opts: *const spdk_nvme_ctrlr_opts,
) {
    let mut nsid = 0;
    let mut num_ns = 0;
    let mut entry: *mut ctrlr_entry = ptr::null_mut();
    let mut ns: *mut spdk_nvme_ns = ptr::null_mut();
    let cdata: *const spdk_nvme_ctrlr_data = spdk_nvme_ctrlr_get_data(ctrlr);
    entry = mem::MaybeUninit::uninitialized().as_mut_ptr();
    if entry.is_null() {
        panic!();
    }
    println!("Attached to {:?}", escape(&(*trid).traddr));
    //println!("{:?} {:?} {:?}", uescape(&(*entry).name), escape(&(*cdata).mn), 0 );

    (*entry).ctrlr.write(*ctrlr);
    (*entry).next.write(g_controllers.ctrlr.get());

    g_controllers.ctrlr.set(*entry);

    /*
     * Each controller has one or more namespaces.  An NVMe namespace is basically
     *  equivalent to a SCSI LUN.  The controller's IDENTIFY data tells us how
     *  many namespaces exist on the controller.  For Intel(R) P3X00 controllers,
     *  it will just be one namespace.
     *
     * Note that in NVMe, namespace IDs start at 1, not 0.
     */
    num_ns = spdk_nvme_ctrlr_get_num_ns(ctrlr);
    println!(
        "Using controller {:?} with {} namespaces.",
        uescape(&(*entry).name),
        num_ns
    );
    for nsid in 1..num_ns + 1 {
        ns = spdk_nvme_ctrlr_get_ns(ctrlr, nsid);
        if (ns.is_null()) {
            continue;
        }
    }
}

pub fn uescape(data: &[u8]) -> String {
    let mut escaped = Vec::with_capacity(data.len() * 4);
    for c in data.iter() {
        match *c as u8 {
            b'\n' => escaped.extend_from_slice(br"\n"),
            b'\r' => escaped.extend_from_slice(br"\r"),
            b'\t' => escaped.extend_from_slice(br"\t"),
            b'"' => escaped.extend_from_slice(b"\\\""),
            b'\\' => escaped.extend_from_slice(br"\\"),
            _ => {
                if (*c as u8) >= 0x20 && (*c as u8) < 0x7f {
                    // c is printable
                    escaped.push(*c as u8);
                } else {
                    escaped.push(b'\\');
                    escaped.push(b'0' + (*c as u8 >> 6));
                    escaped.push(b'0' + ((*c as u8 >> 3) & 7));
                    escaped.push(b'0' + (*c as u8 & 7));
                }
            }
        }
    }
    escaped.shrink_to_fit();
    unsafe { String::from_utf8_unchecked(escaped) }
}

pub fn escape(data: &[i8]) -> String {
    let mut escaped = Vec::with_capacity(data.len() * 4);
    for c in data.iter() {
        match *c as u8 {
            b'\n' => escaped.extend_from_slice(br"\n"),
            b'\r' => escaped.extend_from_slice(br"\r"),
            b'\t' => escaped.extend_from_slice(br"\t"),
            b'"' => escaped.extend_from_slice(b"\\\""),
            b'\\' => escaped.extend_from_slice(br"\\"),
            _ => {
                if (*c as u8) >= 0x20 && (*c as u8) < 0x7f {
                    // c is printable
                    escaped.push(*c as u8);
                } else {
                    escaped.push(b'\\');
                    escaped.push(b'0' + (*c as u8 >> 6));
                    escaped.push(b'0' + ((*c as u8 >> 3) & 7));
                    escaped.push(b'0' + (*c as u8 & 7));
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
        let rc = spdk_nvme_probe(
            ptr::null(),
            ptr::null_mut() as *mut libc::c_void,
            Some(probe_cb),
            Some(attach_cb),
            None,
        );
    }
}
