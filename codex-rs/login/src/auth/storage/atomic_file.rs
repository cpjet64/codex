use std::path::Path;
use tempfile::NamedTempFile;

#[cfg(not(windows))]
pub(super) fn replace_auth_file(temporary: NamedTempFile, auth_file: &Path) -> std::io::Result<()> {
    temporary.persist(auth_file).map_err(|error| error.error)?;
    #[cfg(unix)]
    if let Some(parent) = auth_file.parent() {
        std::fs::File::open(parent)?.sync_all()?;
    }
    Ok(())
}

#[cfg(windows)]
pub(super) fn replace_auth_file(temporary: NamedTempFile, auth_file: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
    use windows_sys::Win32::Storage::FileSystem::REPLACEFILE_WRITE_THROUGH;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    let temporary_path = temporary.into_temp_path().keep()?;
    let source = temporary_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination = auth_file
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let result = if auth_file.exists() {
        unsafe {
            ReplaceFileW(
                destination.as_ptr(),
                source.as_ptr(),
                std::ptr::null(),
                REPLACEFILE_WRITE_THROUGH,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        }
    } else {
        unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_WRITE_THROUGH,
            )
        }
    };
    if result != 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    let _ = std::fs::remove_file(temporary_path);
    Err(error)
}
