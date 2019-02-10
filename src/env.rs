use spdk;
use std::ffi::{c_void, CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

#[derive(Clone)]
pub struct Buf {
    raw: *mut c_void,
}

impl Buf {
    pub fn to_raw(&self) -> *mut c_void {
        self.raw
    }

    pub fn from_raw(raw: *mut c_void) -> Buf {
        Buf { raw: raw }
    }

    /// Fill in the buffer with given content using given fmt
    pub fn fill<S>(&mut self, size: usize, fmt: S, content: S)
    where
        S: Into<String> + Clone,
    {
        let owned_fmt = CString::new(fmt.clone().into()).expect("Couldn't create a string");
        let fmt: *const c_char = owned_fmt.as_ptr();
        let owned_content = CString::new(content.clone().into()).expect("Couldn't create a string");
        let content: *const c_char = owned_content.as_ptr();
        unsafe {
            spdk::snprintf(self.to_raw() as *mut i8, size, fmt, content);
        }
    }

    pub fn read(&self) -> &'static str {
        unsafe { CStr::from_ptr(self.to_raw() as *const i8).to_str().unwrap() }
    }
}

/// spdk_dma_zmalloc()
pub fn dma_zmalloc(size: usize, align: usize) -> Buf {
    let ptr;
    unsafe {
        ptr = spdk::spdk_dma_zmalloc(size, align, ptr::null_mut());
    };
    assert!(!ptr.is_null(), "Failed to malloc");
    Buf { raw: ptr }
}
