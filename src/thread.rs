use spdk;
use std::ffi::{c_void, CStr, CString};
use std::ptr;

use failure::Error;

#[derive(Debug, Fail)]
pub enum ThreadError {
    #[fail(display = "Failed to allocate thread!")]
    ThreadAllocationError(),
}

#[derive(Clone)]
pub struct SpdkIoChannel {
    raw: *mut spdk::spdk_io_channel,
}

impl SpdkIoChannel {
    pub fn from_raw(raw: *mut spdk::spdk_io_channel) -> SpdkIoChannel {
        unsafe { SpdkIoChannel { raw: raw } }
    }

    pub fn to_raw(&self) -> *mut spdk::spdk_io_channel {
        self.raw
    }
}

pub struct SpdkThread {
    raw: *mut spdk::spdk_thread,
}

impl SpdkThread {
    pub fn from_raw(raw: *mut spdk::spdk_thread) -> SpdkThread {
        unsafe { SpdkThread { raw } }
    }
}

pub fn allocate_thread<S>(name: S) -> Result<SpdkThread, Error>
where
    S: Into<String> + Clone,
{
    let name_cstring = CString::new(name.clone().into()).expect("Couldn't create a string");

    let thread_struct = unsafe {
        spdk::spdk_allocate_thread(None, None, None, ptr::null_mut(), name_cstring.as_ptr())
    };
    if thread_struct.is_null() {
        return Err(ThreadError::ThreadAllocationError())?;
    }

    Ok(SpdkThread::from_raw(thread_struct))
}

pub fn free_thread() {
    unsafe {
        spdk::spdk_free_thread();
    }
}

pub fn put_io_channel(channel: SpdkIoChannel) {
    unsafe { spdk::spdk_put_io_channel(channel.to_raw()) }
}
