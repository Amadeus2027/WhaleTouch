use std::path::Path;

pub fn save(path: &Path, key: &str) -> Result<(), String> {
    if key.is_empty() {
        if path.exists() {
            std::fs::remove_file(path).map_err(|_| "无法删除密钥")?;
        }
        return Ok(());
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Security::Cryptography::{
            CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        };
        let mut bytes = key.as_bytes().to_vec();
        let input = CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_mut_ptr(),
        };
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        unsafe {
            if CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            ) == 0
            {
                return Err("Windows 密钥加密失败".into());
            }
            let result = std::fs::write(
                path,
                std::slice::from_raw_parts(out.pbData, out.cbData as usize),
            )
            .map_err(|_| "无法保存密钥".to_string());
            windows_sys::Win32::Foundation::LocalFree(out.pbData as *mut _);
            bytes.fill(0);
            result
        }
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("持久密钥仅支持 Windows DPAPI".into())
    }
}
pub fn load(path: &Path) -> Result<String, String> {
    let mut bytes = std::fs::read(path).map_err(|_| "请先在设置中填写 DeepSeek 官方 API Key")?;
    #[cfg(windows)]
    {
        use windows_sys::Win32::Security::Cryptography::{
            CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        };
        let input = CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_mut_ptr(),
        };
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        unsafe {
            if CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            ) == 0
            {
                return Err("密钥解密失败，请重新保存 API Key".into());
            }
            let key = String::from_utf8(
                std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec(),
            )
            .map_err(|_| "密钥编码无效".to_string());
            std::ptr::write_bytes(out.pbData, 0, out.cbData as usize);
            windows_sys::Win32::Foundation::LocalFree(out.pbData as *mut _);
            key
        }
    }
    #[cfg(not(windows))]
    {
        bytes.fill(0);
        Err("当前预览不读取 API 密钥".into())
    }
}
