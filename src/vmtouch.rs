use log::warn;
use nix::libc;
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

#[derive(Debug)]
pub struct MincoreStat {
    page_size: usize,
    total_pages: usize,
    resident_pages: usize,
}

impl MincoreStat {
    pub fn page_size(&self) -> usize {
        self.page_size
    }

    pub fn total_pages(&self) -> usize {
        self.total_pages
    }

    pub fn resident_pages(&self) -> usize {
        self.resident_pages
    }
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
    NixError(nix::Error),
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

        let mincore_array = unsafe { libc::malloc(pages) } as *mut i8;

        if mincore_array.is_null() {
            Self::_cleanup_mmap(mmap, file_len);
            return Err(Error::AllocError);
        }

        let mincore = unsafe { libc::mincore(mmap, file_len, mincore_array) };
        if mincore != 0 {
            unsafe {
                libc::free(mincore_array as *mut c_void);
            }
            Self::_cleanup_mmap(mmap, file_len);
            return Err(Error::MincoreError);
        }

        Ok(MemoryMap {
            file,
            len: file_len,
            mmap,
            mincore_array,
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

    pub fn resident_pages(&self) -> MincoreStat {
        let resident_pages = (0..self.pages)
            .filter(|&i| (unsafe { self.mincore_array.add(i).read() }) & 0x1 != 0)
            .count();

        MincoreStat {
            page_size: self.page_size,
            total_pages: self.pages,
            resident_pages,
        }
    }

    pub fn touch(&mut self) {
        unsafe {
            for i in 0..self.pages {
                (self.mmap as *mut i8).add(i * self.page_size).read();
                self.mincore_array.add(i).write(1);
            }
        }
    }

    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    pub fn evict(&mut self) -> Result<()> {
        unsafe {
            nix::sys::mman::msync(self.mmap, self.len, nix::sys::mman::MsFlags::MS_INVALIDATE)
        }
        .map_err(Error::NixError)
    }

    #[cfg(target_os = "linux")]
    pub fn evict(&mut self) -> Result<()> {
        unimplemented!()
    }
}
