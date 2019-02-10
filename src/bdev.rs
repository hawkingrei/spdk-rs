/// Note that not all functions belong to "bdev.h" are implemented here.
/// For example, spdk_bdev_open() is implemented in the context instead
/// because spdk_bdev_open works with struct spdk_bdev* and
/// struct spdk_bdev_desc**, which usually used with the context struct.
use std::ffi::{c_void, CStr, CString};
use std::marker;
use std::ptr;

use failure::Error;
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::channel::oneshot::Sender;

#[derive(Debug, Fail)]
pub enum BdevError {
    #[fail(
        display = "Error in write completion({}): {}, offset: {}, length: {}",
        _0, _1, _2, _3
    )]
    WriteError(String, i32, u64, u64),

    #[fail(
        display = "Error in read completion({}): {}, offset: {}, length: {}",
        _0, _1, _2, _3
    )]
    ReadError(String, i32, u64, u64),

    #[fail(display = "Could not find a bdev: {}", _0)]
    NotFound(String),

    #[fail(display = "Could not open device: {}", _0)]
    OpenError(String),

    #[fail(display = "Could not create bdev I/O channel!")]
    IOChannelError(),
}

#[derive(Clone)]
pub struct SpdkBdev {
    raw: *mut raw::spdk_bdev,
}

/// spdk_bdev_get_by_name()
pub fn get_by_name<S>(bdev_name: S) -> Result<SpdkBdev, Error>
where
    S: Into<String> + Clone,
{
    let name_cstring = CString::new(bdev_name.clone().into()).expect("Couldn't create a string");

    let bdev = unsafe { raw::spdk_bdev_get_by_name(name_cstring.as_ptr()) };
    if bdev.is_null() {
        return Err(BdevError::NotFound(bdev_name.clone().into()))?;
    }

    Ok(SpdkBdev::from_raw(bdev))
}

/// spdk_bdev_open()
pub fn open(bdev: SpdkBdev, write: bool, bdev_desc: &mut SpdkBdevDesc) -> Result<(), Error> {
    unsafe {
        let rc = raw::spdk_bdev_open(
            bdev.to_raw(),
            write,
            None,
            ptr::null_mut(),
            bdev_desc.mut_to_raw(),
        );
        match rc != 0 {
            true => Err(BdevError::OpenError(bdev.name().to_string()))?,
            false => Ok(()),
        }
    }
}

/// spdk_bdev_close()
pub fn close(desc: SpdkBdevDesc) {
    unsafe { raw::spdk_bdev_close(desc.to_raw()) }
}

/// spdk_bdev_first()
pub fn first() -> Option<SpdkBdev> {
    unsafe {
        let ptr = raw::spdk_bdev_first();
        if ptr.is_null() {
            None
        } else {
            Some(SpdkBdev::from_raw(ptr))
        }
    }
}

/// spdk_bdev_next()
pub fn next(prev: &SpdkBdev) -> Option<SpdkBdev> {
    unsafe {
        let ptr = raw::spdk_bdev_next(prev.raw);
        if ptr.is_null() {
            None
        } else {
            Some(SpdkBdev::from_raw(ptr))
        }
    }
}

pub fn get_io_channel(desc: SpdkBdevDesc) -> Result<thread::SpdkIoChannel, Error> {
    unsafe {
        let ptr = raw::spdk_bdev_get_io_channel(desc.to_raw());
        if ptr.is_null() {
            Err(BdevError::IOChannelError())?
        } else {
            Ok(thread::SpdkIoChannel::from_raw(ptr))
        }
    }
}

/// spdk_bdev_get_block_size()
pub fn get_block_size(bdev: SpdkBdev) -> u32 {
    unsafe { raw::spdk_bdev_get_block_size(bdev.to_raw()) }
}

/// spdk_bdev_get_buf_align()
pub fn get_buf_align(bdev: SpdkBdev) -> usize {
    unsafe { raw::spdk_bdev_get_buf_align(bdev.to_raw()) }
}

/// spdk_bdev_write()
pub async fn write<'a>(
    desc: SpdkBdevDesc,
    ch: &'a thread::SpdkIoChannel,
    buf: &'a env::Buf,
    offset: u64,
    nbytes: u64,
) -> Result<(), Error> {
    let (sender, receiver) = oneshot::channel();
    let ret: i32;
    unsafe {
        ret = raw::spdk_bdev_write(
            desc.raw,
            ch.to_raw(),
            buf.to_raw(),
            offset,
            nbytes,
            Some(spdk_bdev_io_completion_cb),
            cb_arg::<()>(sender),
        );
    };
    // TODO: we probably need to handle the case where ret != 0
    let res = await!(receiver).expect("Cancellation is not supported");

    match res {
        Ok(()) => Ok(()),
        Err(_e) => Err(BdevError::WriteError(
            desc.spdk_bdev_desc_get_bdev().name().to_string(),
            -1,
            offset,
            nbytes,
        ))?,
    }
}

pub async fn read<'a>(
    desc: SpdkBdevDesc,
    ch: &'a thread::SpdkIoChannel,
    buf: &'a mut env::Buf,
    offset: u64,
    nbytes: u64,
) -> Result<(), Error> {
    let (sender, receiver) = oneshot::channel();
    let ret: i32;
    unsafe {
        ret = raw::spdk_bdev_read(
            desc.raw,
            ch.to_raw(),
            buf.to_raw(),
            offset,
            nbytes,
            Some(spdk_bdev_io_completion_cb),
            cb_arg::<()>(sender),
        );
    };

    // TODO: we probably need to handle the case where ret != 0
    let res = await!(receiver).expect("Cancellation is not supported");

    match res {
        Ok(()) => Ok(()),
        Err(_e) => Err(BdevError::ReadError(
            desc.spdk_bdev_desc_get_bdev().name().to_string(),
            -1,
            offset,
            nbytes,
        ))?,
    }
}

impl SpdkBdev {
    pub fn from_raw(raw: *mut raw::spdk_bdev) -> SpdkBdev {
        unsafe { SpdkBdev { raw: raw } }
    }

    pub fn name(&self) -> &str {
        let str_slice: &str;
        unsafe {
            let c_buf = (*self.raw).name;
            let c_str: &CStr = CStr::from_ptr(c_buf);
            str_slice = c_str.to_str().unwrap();
        }
        str_slice
    }

    pub fn to_raw(&self) -> *mut raw::spdk_bdev {
        self.raw
    }
}

#[derive(Clone)]
pub struct SpdkBdevDesc {
    raw: *mut raw::spdk_bdev_desc,
}

impl SpdkBdevDesc {
    pub fn new() -> SpdkBdevDesc {
        SpdkBdevDesc {
            raw: ptr::null_mut(),
        }
    }

    pub fn from_raw(raw: *mut raw::spdk_bdev_desc) -> SpdkBdevDesc {
        unsafe { SpdkBdevDesc { raw: raw } }
    }

    pub fn to_raw(&self) -> *mut raw::spdk_bdev_desc {
        self.raw
    }

    pub fn mut_to_raw(&mut self) -> *mut *mut raw::spdk_bdev_desc {
        &mut self.raw
    }

    pub fn spdk_bdev_desc_get_bdev(&self) -> SpdkBdev {
        let ptr;
        unsafe {
            ptr = raw::spdk_bdev_desc_get_bdev(self.raw);
        }
        SpdkBdev { raw: ptr }
    }
}

fn cb_arg<T>(sender: Sender<Result<T, i32>>) -> *mut c_void {
    Box::into_raw(Box::new(sender)) as *const _ as *mut c_void
}

extern "C" fn spdk_bdev_io_completion_cb(
    bdev_io: *mut raw::spdk_bdev_io,
    success: bool,
    sender_ptr: *mut c_void,
) {
    let sender = unsafe { Box::from_raw(sender_ptr as *mut Sender<Result<(), i32>>) };
    let ret = if !success { Err(-1) } else { Ok(()) };
    sender.send(ret).expect("Receiver is gone");
}
