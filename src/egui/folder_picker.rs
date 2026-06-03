//! Native folder picker helpers.

use std::path::PathBuf;
#[cfg(windows)]
use windows::Win32::System::Com::CLSCTX_INPROC_SERVER;
#[cfg(windows)]
use windows::Win32::System::Com::COINIT_APARTMENTTHREADED;
#[cfg(windows)]
use windows::Win32::System::Com::CoCreateInstance;
#[cfg(windows)]
use windows::Win32::System::Com::CoInitializeEx;
#[cfg(windows)]
use windows::Win32::System::Com::CoUninitialize;
#[cfg(windows)]
use windows::Win32::UI::Shell::FOS_PICKFOLDERS;
#[cfg(windows)]
use windows::Win32::UI::Shell::FileOpenDialog;
#[cfg(windows)]
use windows::Win32::UI::Shell::IFileOpenDialog;
#[cfg(windows)]
use windows::Win32::UI::Shell::SIGDN_FILESYSPATH;

/// Open a native folder picker.
#[must_use]
#[cfg(windows)]
pub(crate) fn pick_folder() -> Option<PathBuf> {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() };
    let result = unsafe { pick_folder_inner() };
    if initialized {
        unsafe { CoUninitialize() };
    }
    result
}

#[cfg(windows)]
unsafe fn pick_folder_inner() -> Option<PathBuf> {
    let dialog: IFileOpenDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()? };

    let options = unsafe { dialog.GetOptions().ok()? };
    unsafe { dialog.SetOptions(options | FOS_PICKFOLDERS).ok()? };
    unsafe { dialog.Show(None).ok()? };

    let item = unsafe { dialog.GetResult().ok()? };
    let path = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH).ok()? };
    unsafe { path.to_string().ok().map(PathBuf::from) }
}

/// Open a native file picker.
#[must_use]
#[cfg(windows)]
pub(crate) fn pick_file() -> Option<PathBuf> {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() };
    let result = unsafe { pick_file_inner() };
    if initialized {
        unsafe { CoUninitialize() };
    }
    result
}

#[cfg(windows)]
unsafe fn pick_file_inner() -> Option<PathBuf> {
    let dialog: IFileOpenDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()? };

    unsafe { dialog.Show(None).ok()? };

    let item = unsafe { dialog.GetResult().ok()? };
    let path = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH).ok()? };
    unsafe { path.to_string().ok().map(PathBuf::from) }
}

/// Open a native folder picker.
#[must_use]
#[cfg(not(windows))]
pub(crate) fn pick_folder() -> Option<PathBuf> {
    None
}

/// Open a native file picker.
#[must_use]
#[cfg(not(windows))]
pub(crate) fn pick_file() -> Option<PathBuf> {
    None
}
