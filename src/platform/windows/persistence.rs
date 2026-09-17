use std::fs;
use std::io::{self, Write};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Storage::FileSystem::{
    MoveFileExW, ReplaceFileW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    REPLACEFILE_WRITE_THROUGH,
};

/// Atomically writes content to a file on Windows.
///
/// Ensures durable, atomic replacement on NTFS/ReFS by:
/// 1. Creating a temporary file in the target's parent directory.
/// 2. Writing content and flushing buffers with `sync_all()`.
/// 3. Explicitly closing/dropping the file handle before invoking Win32 file APIs.
/// 4. If destination exists, replacing via `ReplaceFileW` (or `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`).
/// 5. If destination does not exist, moving via `MoveFileExW` with `MOVEFILE_WRITE_THROUGH`.
/// 6. Cleaning up the temporary file if any step fails.
pub fn atomic_write_file(path: &Path, content: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        fs::create_dir_all(parent)?;
    }

    let temp_file = tempfile::Builder::new()
        .prefix("sunreactor_atomic_")
        .suffix(".tmp")
        .tempfile_in(parent)?;

    let (mut file, temp_path) = temp_file.into_parts();
    file.write_all(content)?;
    file.sync_all()?;
    // Crucial: close the file handle so Win32 can rename/replace without sharing violations.
    drop(file);

    let wide_temp: Vec<u16> = temp_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let wide_dest: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let res = if path.exists() {
        // ReplaceFileW preserves destination metadata, attributes, and ACLs.
        let replace_res = unsafe {
            ReplaceFileW(
                wide_dest.as_ptr(),
                wide_temp.as_ptr(),
                std::ptr::null(),
                REPLACEFILE_WRITE_THROUGH,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if replace_res != 0 {
            1
        } else {
            // Fall back to MoveFileExW if ReplaceFileW fails (e.g. specific filesystem variations)
            unsafe {
                MoveFileExW(
                    wide_temp.as_ptr(),
                    wide_dest.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            }
        }
    } else {
        unsafe {
            MoveFileExW(
                wide_temp.as_ptr(),
                wide_dest.as_ptr(),
                MOVEFILE_WRITE_THROUGH,
            )
        }
    };

    if res == 0 {
        let err = unsafe { GetLastError() };
        let _ = fs::remove_file(&temp_path);
        return Err(io::Error::from_raw_os_error(err as i32));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_creates_and_overwrites_repeatedly() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let target = temp_dir.path().join("test_config.toml");

        // 1. Initial write
        atomic_write_file(&target, b"version = 1\n").expect("initial write");
        assert_eq!(
            fs::read_to_string(&target).expect("read initial"),
            "version = 1\n"
        );

        // 2. Overwrite existing
        atomic_write_file(&target, b"version = 2\n").expect("overwrite");
        assert_eq!(
            fs::read_to_string(&target).expect("read overwrite"),
            "version = 2\n"
        );

        // 3. Repeated overwrite
        for i in 3..=10 {
            let content = format!("version = {i}\n");
            atomic_write_file(&target, content.as_bytes()).expect("repeated overwrite");
            assert_eq!(fs::read_to_string(&target).expect("read repeated"), content);
        }
    }

    #[test]
    fn atomic_write_handles_unicode_path() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let target = temp_dir
            .path()
            .join("Ayarlar ☀️ & Türkçe Karakterler (çşğüıö).toml");

        atomic_write_file(&target, b"test = true\n").expect("write unicode path");
        assert_eq!(
            fs::read_to_string(&target).expect("read unicode"),
            "test = true\n"
        );
    }
}
