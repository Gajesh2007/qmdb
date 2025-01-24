use std::fs::remove_file;
use std::os::unix::fs::OpenOptionsExt;
use std::alloc::{alloc, dealloc, Layout};

#[cfg(feature = "in_sp1")]
use hpfile::file::File;
#[cfg(not(feature = "in_sp1"))]
use std::{fs::File, io::Write, os::unix::fs::FileExt};

// #![allow(dead_code)]

// Block size for direct I/O alignment
const BLOCK_SIZE: usize = 4096;

pub struct TempFile {
    file: File,
    fname: String,
    use_direct_io: bool,
}

impl TempFile {
    pub fn new(fname: String) -> Self {
        Self::with_options(fname, false)
    }

    pub fn with_options(fname: String, use_direct_io: bool) -> Self {
        let mut options = File::options();
        options.create(true).read(true).write(true);
        
        if use_direct_io {
            #[cfg(target_os = "linux")]
            options.custom_flags(libc::O_DIRECT);
            #[cfg(target_os = "macos")]
            options.custom_flags(0x40_0000); // O_DIRECT on macOS
        }

        match options.open(fname.clone()) {
            Ok(file) => Self { file, fname, use_direct_io },
            Err(_) => panic!("Fail to open file: {}", fname),
        }
    }

    pub fn get_name(&self) -> String {
        self.fname.clone()
    }

    // Align buffer for direct I/O if needed
    fn create_aligned_buffer(&self, size: usize) -> Option<(Box<[u8]>, Layout)> {
        if !self.use_direct_io {
            return None;
        }

        let aligned_size = (size + BLOCK_SIZE - 1) & !(BLOCK_SIZE - 1);
        let layout = Layout::from_size_align(aligned_size, BLOCK_SIZE).unwrap();
        
        unsafe {
            let ptr = alloc(layout);
            let aligned_buf = Box::from_raw(std::slice::from_raw_parts_mut(ptr, aligned_size));
            Some((aligned_buf, layout))
        }
    }

    pub fn read_at(&self, buf: &mut [u8], off: u64) -> usize {
        if buf.is_empty() {
            return 0;
        }

        if self.use_direct_io {
            let (mut aligned_buf, layout) = self.create_aligned_buffer(buf.len()).unwrap();
            let read_size = self.file.read_at(&mut aligned_buf, off).unwrap();
            buf.copy_from_slice(&aligned_buf[..buf.len()]);
            let ptr = Box::into_raw(aligned_buf) as *mut u8;
            unsafe {
                dealloc(ptr, layout);
            }
            read_size
        } else {
            self.file.read_at(buf, off).unwrap()
        }
    }

    pub fn write(&mut self, buf: &[u8]) {
        if buf.is_empty() {
            return;
        }

        if self.use_direct_io {
            let (mut aligned_buf, layout) = self.create_aligned_buffer(buf.len()).unwrap();
            aligned_buf[..buf.len()].copy_from_slice(buf);
            self.file.write_all(&aligned_buf).unwrap();
            let ptr = Box::into_raw(aligned_buf) as *mut u8;
            unsafe {
                dealloc(ptr, layout);
            }
        } else {
            self.file.write(buf).unwrap();
        }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Err(_) = remove_file(self.fname.clone()) {
            panic!("Fail to remove file: {}", self.fname);
        }
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn test_tempfile_new() {
        let fname = "./test_file.txt".to_string();
        let temp_file = TempFile::new(fname.clone());
        assert_eq!(temp_file.fname, fname);
    }

    #[test]
    #[serial]
    fn test_tempfile_write_and_read_at() {
        let fname = "./test_file.txt".to_string();
        let mut temp_file = TempFile::new(fname.clone());
        {
            let mut buf = [0u8; 5];
            temp_file.write(b"Hello");
            let bytes_read = temp_file.read_at(&mut buf, 0);
            assert_eq!(bytes_read, 5);
            assert_eq!(&buf, b"Hello");
        }
        {
            let buf = b"World";
            temp_file.write(buf);
            let mut read_buf = [0u8; 5];
            let bytes_read = temp_file.read_at(&mut read_buf, 5);
            assert_eq!(bytes_read, 5);
            assert_eq!(&read_buf, buf);
        }
    }

    #[test]
    #[serial]
    fn test_tempfile_drop() {
        let fname = "./test_file.txt".to_string();
        {
            let _temp_file = TempFile::new(fname.clone());
        }
        assert!(!std::path::Path::new(&fname).exists());
    }
}
