use spdk;
use std::ptr;

pub struct SpdkBdevIO {
    raw: *mut raw::spdk_bdev_io,
}

impl SpdkBdevIO {
    pub fn from_raw(raw: *mut spdk::spdk_bdev_io) -> SpdkBdevIO {
        unsafe { SpdkBdevIO { raw: raw } }
    }

    pub fn to_raw(&self) -> *mut spdk::spdk_bdev_io {
        self.raw
    }

    pub fn new() -> Self {
        SpdkBdevIO {
            raw: ptr::null_mut(),
        }
    }
}
