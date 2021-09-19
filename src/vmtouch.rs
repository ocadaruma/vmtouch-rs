use log::warn;
use nix::libc;
use nix::sys::mman;
use nix::unistd;
use std::ffi::c_void;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::ptr::null_mut;

#[cfg(target_os = "linux")]
type MincoreChar = u8;

#[cfg(target_os = "macos")]
type MincoreChar = i8;

pub struct MappedFile {
    file: File,
    len: usize,
    mmap: *mut c_void,
    mincore_array: *mut MincoreChar,
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

impl Drop for MappedFile {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.mincore_array as *mut c_void);
            Self::_cleanup_mmap(self.mmap, self.len);
        }
    }
}

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    NotPageAligned,
    AllocFailed,
    Nix(nix::Error),
}

pub type Result<T> = core::result::Result<T, Error>;

impl MappedFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<MappedFile> {
        let page_size = unistd::sysconf(unistd::SysconfVar::PAGE_SIZE)
            .ok()
            .flatten()
            .expect("Failed to get page size") as usize;
        let file = File::open(path).map_err(Error::IO)?;
        let file_meta = file.metadata().map_err(Error::IO)?;
        let file_len = file_meta.len() as usize;

        let mmap = unsafe {
            mman::mmap(
                null_mut(),
                file_len,
                mman::ProtFlags::PROT_READ,
                mman::MapFlags::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        }
        .map_err(Error::Nix)?;

        if (mmap as i64 & (page_size - 1) as i64) != 0 {
            Self::_cleanup_mmap(mmap, file_len);
            return Err(Error::NotPageAligned);
        }

        let pages = (file_len + page_size + 1) / page_size;

        let mincore_array = unsafe { libc::malloc(pages) } as *mut MincoreChar;

        if mincore_array.is_null() {
            Self::_cleanup_mmap(mmap, file_len);
            return Err(Error::AllocFailed);
        }

        unsafe {
            if let Err(err) = nix::Error::result(libc::mincore(mmap, file_len, mincore_array)) {
                libc::free(mincore_array as *mut c_void);
                Self::_cleanup_mmap(mmap, file_len);
                return Err(Error::Nix(err));
            }
        };

        Ok(MappedFile {
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
            if let Err(err) = mman::munmap(mmap, len) {
                warn!("failed to unmap. error: {}", err.desc());
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

    #[cfg(target_os = "macos")]
    pub fn evict(&mut self) -> Result<()> {
        unsafe {
            nix::sys::mman::msync(self.mmap, self.len, nix::sys::mman::MsFlags::MS_INVALIDATE)
        }
        .map_err(Error::Nix)
    }

    #[cfg(target_os = "linux")]
    pub fn evict(&mut self) -> Result<()> {
        match nix::fcntl::posix_fadvise(
            self.file.as_raw_fd(),
            0,
            self.len as i64,
            nix::fcntl::PosixFadviseAdvice::POSIX_FADV_DONTNEED,
        ) {
            Ok(ret) if ret == 0 => Ok(()),
            Ok(ret) => Err(Error::Nix(nix::Error::from_i32(ret))),
            Err(err) => Err(Error::Nix(err)),
        }
    }
}
