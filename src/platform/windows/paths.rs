use std::env;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use windows_sys::core::GUID;
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::UI::Shell::SHGetKnownFolderPath;

const APP_DIR_NAME: &str = "SunReactor";
const CONFIG_FILE_NAME: &str = "config.toml";
const STATE_FILE_NAME: &str = "runtime-state.json";

// FOLDERID_LocalAppData: {F1B32785-6FBA-4FCF-9D55-7B8E7F157091}
const FOLDERID_LOCAL_APP_DATA: GUID = GUID {
    data1: 0xF1B3_2785,
    data2: 0x6FBA,
    data3: 0x4FCF,
    data4: [0x9D, 0x55, 0x7B, 0x8E, 0x7F, 0x15, 0x70, 0x91],
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WindowsPathError {
    #[error("SHGetKnownFolderPath failed with HRESULT 0x{hresult:08X}")]
    KnownFolderFailed { hresult: i32 },
    #[error("failed to resolve local AppData directory")]
    MissingLocalAppData,
}

/// Resolves the base directory for SunReactor data (%LOCALAPPDATA%\SunReactor).
///
/// If `SUNREACTOR_TEST_ROOT` is set in the environment (e.g. during integration tests),
/// that directory is used instead of the user's real AppData directory.
pub fn local_app_data_dir() -> Result<PathBuf, WindowsPathError> {
    if let Some(test_root) = env::var_os("SUNREACTOR_TEST_ROOT") {
        if !test_root.is_empty() {
            return Ok(PathBuf::from(test_root).join(APP_DIR_NAME));
        }
    }

    let mut path_ptr: *mut u16 = std::ptr::null_mut();
    // Safety justification:
    // - Pointer validity: `path_ptr` receives a pointer to a null-terminated UTF-16 string allocated by the shell.
    // - Ownership: When S_OK (0) is returned, caller must free `path_ptr` via `CoTaskMemFree`.
    // - Lifetime: We copy into an owned `OsString` immediately and release the allocation in the same block.
    let hr = unsafe {
        SHGetKnownFolderPath(
            &FOLDERID_LOCAL_APP_DATA,
            0,
            std::ptr::null_mut(), // Current user token
            &raw mut path_ptr,
        )
    };

    if hr != 0 || path_ptr.is_null() {
        return Err(WindowsPathError::KnownFolderFailed { hresult: hr });
    }

    let os_string = unsafe {
        let mut len = 0;
        while *path_ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(path_ptr, len);
        let os = OsString::from_wide(slice);
        CoTaskMemFree(path_ptr.cast());
        os
    };

    let base = PathBuf::from(os_string);
    Ok(base.join(APP_DIR_NAME))
}

pub fn config_dir() -> Result<PathBuf, WindowsPathError> {
    local_app_data_dir()
}

pub fn config_file() -> Result<PathBuf, WindowsPathError> {
    config_dir().map(|p| p.join(CONFIG_FILE_NAME))
}

pub fn state_dir() -> Result<PathBuf, WindowsPathError> {
    local_app_data_dir().map(|p| p.join("state"))
}

pub fn state_file() -> Result<PathBuf, WindowsPathError> {
    state_dir().map(|p| p.join(STATE_FILE_NAME))
}

pub fn cache_dir() -> Result<PathBuf, WindowsPathError> {
    local_app_data_dir().map(|p| p.join("cache"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_root_override_produces_deterministic_paths() {
        let _guard = TEST_LOCK.lock().unwrap();
        let fake_root = Path::new(r"C:\TestEnvironment\FakeUser\AppData\Local");
        env::set_var("SUNREACTOR_TEST_ROOT", fake_root);

        let cfg_dir = config_dir().expect("config dir with test root");
        let cfg_file = config_file().expect("config file with test root");
        let st_dir = state_dir().expect("state dir with test root");
        let st_file = state_file().expect("state file with test root");
        let c_dir = cache_dir().expect("cache dir with test root");

        assert_eq!(cfg_dir, fake_root.join("SunReactor"));
        assert_eq!(cfg_file, fake_root.join("SunReactor").join("config.toml"));
        assert_eq!(st_dir, fake_root.join("SunReactor").join("state"));
        assert_eq!(
            st_file,
            fake_root
                .join("SunReactor")
                .join("state")
                .join("runtime-state.json")
        );
        assert_eq!(c_dir, fake_root.join("SunReactor").join("cache"));

        env::remove_var("SUNREACTOR_TEST_ROOT");
    }

    #[test]
    fn unicode_and_spaces_in_test_root() {
        let _guard = TEST_LOCK.lock().unwrap();
        let fake_root = Path::new(r"C:\Kullanıcılar\Deneme Kullanıcısı ☀️\AppData\Local");
        env::set_var("SUNREACTOR_TEST_ROOT", fake_root);

        let cfg_file = config_file().expect("config file with unicode test root");
        assert!(cfg_file.to_string_lossy().contains("Deneme Kullanıcısı ☀️"));

        env::remove_var("SUNREACTOR_TEST_ROOT");
    }
}
