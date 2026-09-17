use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY,
    TOKEN_USER,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

pub struct AutoHandle(pub HANDLE);

unsafe impl Send for AutoHandle {}
unsafe impl Sync for AutoHandle {}

impl std::fmt::Debug for AutoHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AutoHandle({:p})", self.0)
    }
}

impl Drop for AutoHandle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

pub struct SecurityDescriptorGuard {
    sd: PSECURITY_DESCRIPTOR,
}

impl Drop for SecurityDescriptorGuard {
    fn drop(&mut self) {
        if !self.sd.is_null() {
            unsafe {
                LocalFree(self.sd.cast());
            }
        }
    }
}

/// Computes a deterministic 32-character hexadecimal digest of a user SID.
///
/// This avoids exposing the raw domain/account SID in public pipe names
/// while ensuring each Windows user gets a unique, collision-free control endpoint.
#[must_use]
pub fn hash_sid(sid: &str) -> String {
    let mut h1: u64 = 0xcbf2_9ce4_8422_2325;
    let mut h2: u64 = 0x8422_2325_cbf2_9ce4;
    for byte in sid.as_bytes() {
        h1 ^= u64::from(*byte);
        h1 = h1.wrapping_mul(0x100_0000_01b3);
        h2 ^= u64::from(!*byte);
        h2 = h2.wrapping_mul(0x100_0000_01b3);
    }
    format!("{h1:016x}{h2:016x}")
}

/// Retrieves the current process user's Windows Security Identifier (SID) as a string.
pub fn get_current_user_sid() -> Result<String, String> {
    unsafe {
        let process = GetCurrentProcess();
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(process, TOKEN_QUERY, &raw mut token) == 0 {
            return Err(format!("OpenProcessToken failed: {}", GetLastError()));
        }
        let _token_guard = AutoHandle(token);

        let mut len = 0;
        let _ = GetTokenInformation(token, TokenUser, null_mut(), 0, &raw mut len);
        if len == 0 {
            return Err(format!(
                "GetTokenInformation size query failed: {}",
                GetLastError()
            ));
        }

        // Align allocation for TOKEN_USER struct (8-byte alignment)
        let mut buf = vec![0u64; (len as usize).div_ceil(8)];
        let buf_ptr = buf.as_mut_ptr().cast::<u8>();
        if GetTokenInformation(token, TokenUser, buf_ptr.cast(), len, &raw mut len) == 0 {
            return Err(format!(
                "GetTokenInformation data failed: {}",
                GetLastError()
            ));
        }

        let token_user = &*buf.as_ptr().cast::<TOKEN_USER>();
        let mut string_sid_ptr: *mut u16 = null_mut();
        if windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW(
            token_user.User.Sid,
            &raw mut string_sid_ptr,
        ) == 0
        {
            return Err(format!("ConvertSidToStringSidW failed: {}", GetLastError()));
        }

        let mut str_len = 0;
        while *string_sid_ptr.add(str_len) != 0 {
            str_len += 1;
        }
        let slice = std::slice::from_raw_parts(string_sid_ptr, str_len);
        let sid = String::from_utf16_lossy(slice);
        LocalFree(string_sid_ptr.cast());
        Ok(sid)
    }
}

/// Creates a security descriptor granting full access (Generic All) ONLY to the current user
/// and Local SYSTEM (`SY`). All other local and remote callers are denied.
pub fn create_pipe_security_attributes(
    user_sid: &str,
) -> Result<(SECURITY_ATTRIBUTES, SecurityDescriptorGuard), String> {
    // SDDL structure:
    // D: = Discretionary Access Control List (DACL)
    // (A;;GA;;;{user_sid}) = Allow Generic All to current user
    // (A;;GA;;;SY) = Allow Generic All to Local System
    let sddl = format!("D:(A;;GA;;;{user_sid})(A;;GA;;;SY)");
    let wide_sddl: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0)).collect();

    let mut sd: PSECURITY_DESCRIPTOR = null_mut();
    let res = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide_sddl.as_ptr(),
            1, // SDDL_REVISION_1
            &raw mut sd,
            null_mut(),
        )
    };
    if res == 0 {
        return Err(format!(
            "ConvertStringSecurityDescriptorToSecurityDescriptorW failed: {}",
            unsafe { GetLastError() }
        ));
    }

    let sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd,
        bInheritHandle: 0,
    };
    Ok((sa, SecurityDescriptorGuard { sd }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_sid_is_deterministic_and_unique() {
        let sid_alice = "S-1-5-21-3623811015-3361044348-30300820-1013";
        let sid_bob = "S-1-5-21-3623811015-3361044348-30300820-1014";

        let hash_alice_1 = hash_sid(sid_alice);
        let hash_alice_2 = hash_sid(sid_alice);
        let hash_bob = hash_sid(sid_bob);

        assert_eq!(hash_alice_1, hash_alice_2);
        assert_ne!(hash_alice_1, hash_bob);
        assert_eq!(hash_alice_1.len(), 32);
        assert!(hash_alice_1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn sddl_string_syntax_and_dacl_construction() {
        let sid = "S-1-5-21-123456789-987654321-123456-1001";
        let result = create_pipe_security_attributes(sid);
        assert!(
            result.is_ok(),
            "SDDL parser should succeed for standard SID format"
        );
        let (sa, _guard) = result.unwrap();
        assert_eq!(
            sa.nLength,
            std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32
        );
        assert_eq!(sa.bInheritHandle, 0);
        assert!(!sa.lpSecurityDescriptor.is_null());
    }
}
