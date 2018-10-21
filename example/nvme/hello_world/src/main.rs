#![feature(maybe_uninit)]
extern crate libc;
extern crate spdk_sys;
#[macro_use]
extern crate lazy_static;

use spdk_sys::*;
use std::cell::Cell;
use std::ffi::CString;
use std::mem;
use std::mem::drop;
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
    ns_entry: *mut ns_entry,
    using_cmb_io: u8,
    is_completed: u8,
}

unsafe fn register_ns(ctrlr: *mut spdk_nvme_ctrlr, ns: *mut spdk_nvme_ns) {
    let mut entry: *mut ns_entry = ptr::null_mut();
    let cdata: *const spdk_nvme_ctrlr_data = spdk_nvme_ctrlr_get_data(ctrlr);
    entry = mem::MaybeUninit::uninitialized().as_mut_ptr();
    if entry.is_null() {
        panic!();
    }

    if (!spdk_nvme_ns_is_active(ns)) {
        println!(
            "Controller: Skipping inactive NS {}",
            spdk_nvme_ns_get_id(ns)
        );
        return;
    }

    (*entry).ctrlr = ctrlr;
    (*entry).ns = ns;
    //(*entry).next = g_namespaces.g_namespaces.get();
    (*entry).next = ptr::null_mut() as *mut ns_entry;
    g_namespaces.g_namespaces.set(entry);

    println!(
        "  Namespace ID: {} size: {}GB\n",
        spdk_nvme_ns_get_id(ns),
        spdk_nvme_ns_get_size(ns) / 1000000000,
    );
}

unsafe extern "C" fn read_complete(arg: *mut libc::c_void, completion: *const spdk_nvme_cpl) {
    let sequence: *mut hello_world_sequence = arg as *mut hello_world_sequence;
    println!(
        "{}",
        String::from_utf8_unchecked(Vec::from_raw_parts(
            (*sequence).buf as *mut u8,
            0x1000,
            0x1000
        ))
    );
    spdk_free((*sequence).buf);
    (*sequence).is_completed = 1;
}

unsafe extern "C" fn write_complete(arg: *mut libc::c_void, completion: *const spdk_nvme_cpl) {
    let sequence: *mut hello_world_sequence = arg as *mut hello_world_sequence;
    let mut ns_entry: *mut ns_entry = (*sequence).ns_entry;
    let mut rc = 0;

    /*
     * The write I/O has completed.  Free the buffer associated with
     *  the write I/O and allocate a new zeroed buffer for reading
     *  the data back from the NVMe namespace.
     */
    if ((*sequence).using_cmb_io == 1) {
        spdk_nvme_ctrlr_free_cmb_io_buffer((*ns_entry).ctrlr, (*sequence).buf, 0x1000);
    } else {
        spdk_free((*sequence).buf);
    }
    (*sequence).buf = spdk_zmalloc(
        0x1000,
        0x1000,
        ptr::null_mut() as *mut u64,
        SPDK_ENV_SOCKET_ID_ANY,
        SPDK_MALLOC_DMA,
    );

    rc = spdk_nvme_ns_cmd_read(
        (*ns_entry).ns,
        (*ns_entry).qpair,
        (*sequence).buf,
        0, /* LBA start */
        1, /* number of LBAs */
        Some(read_complete),
        sequence as *mut libc::c_void,
        0,
    );
    if (rc != 0) {
        println!("starting read I/O failed");
        process::exit(1);
    }
}

unsafe extern "C" fn probe_cb(
    cb_ctx: *mut libc::c_void,
    trid: *const spdk_nvme_transport_id,
    opts: *mut spdk_nvme_ctrlr_opts,
) -> bool {
    println!(
        "{:?}",
        CString::from_vec_unchecked(tovecu8((*trid).traddr.to_vec()))
    );
    return true;
}

unsafe fn tovecu8(mut veci8: Vec<i8>) -> Vec<u8> {
    let length = veci8.len() * mem::size_of::<i8>();
    let capacity = veci8.capacity() * mem::size_of::<i8>();
    let ptr = veci8.as_mut_ptr() as *mut u8;
    mem::forget(veci8);
    Vec::from_raw_parts(ptr, length, capacity)
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
    println!(
        "Attached to {:?}",
        CString::from_vec_unchecked(tovecu8((*trid).traddr.to_vec()))
    );
    let tmp_mn = tovecu8((*cdata).mn.to_vec());
    let tmp_sn = tovecu8((*cdata).sn.to_vec());
    libc::snprintf(
        (*entry).name.as_ptr() as *mut i8,
        mem::size_of::<[u8; 1024]>(),
        CString::new("%-20.20s (%-20.20s)").unwrap().as_ptr(),
        CString::from_vec_unchecked(tmp_mn).as_ptr(),
        CString::from_vec_unchecked(tmp_sn).as_ptr(),
    );

    (*entry).ctrlr = ctrlr;
    (*entry).next = ptr::null_mut() as *mut ctrlr_entry;
    println!((*entry).ctrlr)
    g_controllers.ctrlr.set(entry);

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
        CString::from_vec_unchecked((*entry).name.to_vec()),
        num_ns
    );
    for nsid in 1..num_ns + 1 {
        ns = spdk_nvme_ctrlr_get_ns(ctrlr, nsid);
        if (ns.is_null()) {
            continue;
        }
        register_ns(ctrlr, ns);
    }
}

unsafe fn hello_world() {
    let mut ns_entry: *mut ns_entry = g_namespaces.g_namespaces.get();
    let mut sequence: hello_world_sequence;
    let mut rc = 0;
    sequence = mem::uninitialized();
    while (!ns_entry.is_null()) {
        /*
         * Allocate an I/O qpair that we can use to submit read/write requests
         *  to namespaces on the controller.  NVMe controllers typically support
         *  many qpairs per controller.  Any I/O qpair allocated for a controller
         *  can submit I/O to any namespace on that controller.
         *
         * The SPDK NVMe driver provides no synchronization for qpair accesses -
         *  the application must ensure only a single thread submits I/O to a
         *  qpair, and that same thread must also check for completions on that
         *  qpair.  This enables extremely efficient I/O processing by making all
         *  I/O operations completely lockless.
         */

        (*ns_entry).qpair = spdk_nvme_ctrlr_alloc_io_qpair((*ns_entry).ctrlr, ptr::null(), 0);
        if ((*ns_entry).qpair.is_null()) {
            println!("ERROR: spdk_nvme_ctrlr_alloc_io_qpair() failed");
            return;
        }

        /*
         * Use spdk_dma_zmalloc to allocate a 4KB zeroed buffer.  This memory
         * will be pinned, which is required for data buffers used for SPDK NVMe
         * I/O operations.
         */
        sequence.using_cmb_io = 1;
        sequence.buf = spdk_nvme_ctrlr_alloc_cmb_io_buffer((*ns_entry).ctrlr, 0x1000);

        if (sequence.buf.is_null()) {
            sequence.using_cmb_io = 0;
            sequence.buf = spdk_zmalloc(
                0x1000,
                0x1000,
                ptr::null_mut() as *mut u64,
                SPDK_ENV_SOCKET_ID_ANY,
                SPDK_MALLOC_DMA,
            );
        }
        if (sequence.buf.is_null()) {
            println!("ERROR: write buffer allocation failed");
            return;
        }
        if (sequence.using_cmb_io == 0) {
            println!("INFO: using controller memory buffer for IO");
        } else {
            println!("INFO: using host memory buffer for IO");
        }
        sequence.is_completed = 0;
        sequence.ns_entry = ns_entry;
        /*
         * Print "Hello world!" to sequence.buf.  We will write this data to LBA
         *  0 on the namespace, and then later read it back into a separate buffer
         *  to demonstrate the full I/O path.
         */
        libc::snprintf(
            sequence.buf as *mut i8,
            0x1000,
            CString::new("%s").unwrap().as_ptr(),
            CString::new("Hello world!\n").unwrap().as_ptr(),
        );
        /*
         * Write the data buffer to LBA 0 of this namespace.  "write_complete" and
         *  "&sequence" are specified as the completion callback function and
         *  argument respectively.  write_complete() will be called with the
         *  value of &sequence as a parameter when the write I/O is completed.
         *  This allows users to potentially specify different completion
         *  callback routines for each I/O, as well as pass a unique handle
         *  as an argument so the application knows which I/O has completed.
         *
         * Note that the SPDK NVMe driver will only check for completions
         *  when the application calls spdk_nvme_qpair_process_completions().
         *  It is the responsibility of the application to trigger the polling
         *  process.
         */
        rc = spdk_nvme_ns_cmd_read(
            (*ns_entry).ns,
            (*ns_entry).qpair,
            sequence.buf,
            0, /* LBA start */
            1, /* number of LBAs */
            Some(write_complete),
            &sequence as *const _ as *mut libc::c_void,
            0,
        );
        if (rc != 0) {
            println!("starting write I/O failed");
            process::exit(1);
        }
        /*
         * Poll for completions.  0 here means process all available completions.
         *  In certain usage models, the caller may specify a positive integer
         *  instead of 0 to signify the maximum number of completions it should
         *  process.  This function will never block - if there are no
         *  completions pending on the specified qpair, it will return immediately.
         *
         * When the write I/O completes, write_complete() will submit a new I/O
         *  to read LBA 0 into a separate buffer, specifying read_complete() as its
         *  completion routine.  When the read I/O completes, read_complete() will
         *  print the buffer contents and set sequence.is_completed = 1.  That will
         *  break this loop and then exit the program.
         */
        while (sequence.is_completed == 0) {
            spdk_nvme_qpair_process_completions((*ns_entry).qpair, 0);
        }
        /*
         * Free the I/O qpair.  This typically is done when an application exits.
         *  But SPDK does support freeing and then reallocating qpairs during
         *  operation.  It is the responsibility of the caller to ensure all
         *  pending I/O are completed before trying to free the qpair.
         */
        spdk_nvme_ctrlr_free_io_qpair((*ns_entry).qpair);
        ns_entry = (*ns_entry).next;
    }
}

unsafe fn cleanup() {
    let mut ns_entry: *mut ns_entry = g_namespaces.g_namespaces.get();
    let mut ctrlr_entry: *mut ctrlr_entry = *(g_controllers.ctrlr.as_ptr());

    while (!ns_entry.is_null()) {
        let mut next: *mut ns_entry = (*ns_entry).next;
        drop(ns_entry);
        ns_entry = next;
    }

    while (!ctrlr_entry.is_null()) {
        let mut next: *mut ctrlr_entry = (*ctrlr_entry).next;
        println!((*ctrlr_entry).ctrlr)
        spdk_nvme_detach((*ctrlr_entry).ctrlr);
        drop(ctrlr_entry);
        ctrlr_entry = next;
    }
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

        if (rc != 0) {
            println!("spdk_nvme_probe() failed");
            cleanup();
            return;
        }

        if (g_controllers.ctrlr.get().is_null()) {
            println!("no NVMe controllers found");
            cleanup();
            return;
        }

        println!("Initialization complete.");
        hello_world();
        cleanup();
        return;
    }
}
