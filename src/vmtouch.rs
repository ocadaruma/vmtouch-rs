use log::warn;
use std::ffi::{c_void, CStr};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::ptr::null_mut;

pub struct MemoryMap {
    file: File,
    len: usize,
    mmap: *mut c_void,
    mincore_array: *mut i8,
    page_size: usize,
    pages: usize,
}

impl Drop for MemoryMap {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.mincore_array as *mut c_void);
            Self::_cleanup_mmap(self.mmap, self.len);
        }
    }
}

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    MmapError,
    AllocError,
    MincoreError,
}

pub type Result<T> = core::result::Result<T, Error>;

impl MemoryMap {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<MemoryMap> {
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
        let file = File::open(path).map_err(Error::IOError)?;
        let file_meta = file.metadata().map_err(Error::IOError)?;
        let file_len = file_meta.len() as usize;

        let mmap = unsafe {
            libc::mmap(
                null_mut(),
                file_len,
                libc::PROT_READ,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        };

        if mmap == libc::MAP_FAILED ||
            // check alignment
            (mmap as i64 & (page_size - 1) as i64) != 0
        {
            Self::_cleanup_mmap(mmap, file_len);
            return Err(Error::MmapError);
        }

        let pages = (file_len + page_size + 1) / page_size;

        let mincore_array = unsafe { libc::malloc(pages) };

        if mincore_array.is_null() {
            Self::_cleanup_mmap(mmap, file_len);
            return Err(Error::AllocError);
        }

        Ok(MemoryMap {
            file,
            len: file_len,
            mmap,
            mincore_array: mincore_array as *mut i8,
            page_size,
            pages,
        })
    }

    fn _cleanup_mmap(mmap: *mut c_void, len: usize) {
        unsafe {
            let unmap = libc::munmap(mmap, len);
            if unmap != 0 {
                warn!(
                    "failed to unmap. error: {:?}",
                    CStr::from_ptr(libc::strerror(unmap))
                );
            }
        }
    }

    pub fn pages(&self) -> usize {
        self.pages
    }

    pub fn resident_pages(&self) -> Result<usize> {
        let mincore = unsafe { libc::mincore(self.mmap, self.len, self.mincore_array) };
        if mincore != 0 {
            return Err(Error::MincoreError);
        }

        Ok((0..self.pages)
            .filter(|&i| (unsafe { *(self.mincore_array.add(i)) }) & 0x1 != 0)
            .count())
    }

    pub fn touch(&mut self) -> Result<()> {
        unimplemented!()
    }

    pub fn evict(&mut self) -> Result<()> {
        unimplemented!()
    }
}
