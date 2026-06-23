use eyre::Context;
use std::collections::HashMap;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::path::PathBuf;
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::COINIT_APARTMENTTHREADED;
use windows::Win32::System::Com::CoInitializeEx;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Com::CoUninitialize;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::IShellFolder;
use windows::Win32::UI::Shell::SHBindToParent;
use windows::Win32::UI::Shell::SHOpenFolderAndSelectItems;
use windows::Win32::UI::Shell::SHParseDisplayName;
use windows::core::PCWSTR;

/// Opens Explorer windows and selects the specified items, grouped by parent directory.
pub fn open_folder_and_select_items<P: AsRef<Path>>(paths: &[P]) -> eyre::Result<()> {
    let _com_guard = ComGuard::new()?;
    let mut grouped: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

    for path in paths {
        let path = dunce::canonicalize(path.as_ref())?;
        let parent = path
            .parent()
            .ok_or_else(|| eyre::eyre!("Path has no parent: {}", path.display()))?
            .to_path_buf();
        grouped.entry(parent).or_default().push(path);
    }

    for (parent_path, child_paths) in grouped {
        select_items_in_folder(&parent_path, &child_paths)?;
    }

    Ok(())
}

fn select_items_in_folder(parent_path: &Path, child_paths: &[PathBuf]) -> eyre::Result<()> {
    let pidl_parent = Pidl::try_new(parent_path)?;
    let mut full_pidls: Vec<Pidl> = Vec::with_capacity(child_paths.len());

    for child_path in child_paths {
        full_pidls.push(Pidl::try_new(child_path)?);
    }

    let apidl: Vec<*const ITEMIDLIST> = full_pidls
        .iter()
        .map(Pidl::child_pidl)
        .collect::<eyre::Result<Vec<_>>>()?
        .iter()
        .map(|pidl| pidl.as_ptr())
        .collect();

    unsafe {
        SHOpenFolderAndSelectItems(pidl_parent.as_ptr() as _, Some(&apidl), 0)
            .wrap_err("Failed to open Explorer selection")?;
    }

    Ok(())
}

struct ComGuard {
    should_uninitialize: bool,
}

impl ComGuard {
    fn new() -> eyre::Result<Self> {
        unsafe {
            let result = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if result.is_ok() {
                Ok(Self {
                    should_uninitialize: true,
                })
            } else if result == RPC_E_CHANGED_MODE {
                Ok(Self {
                    should_uninitialize: false,
                })
            } else {
                Err(windows::core::Error::from(result).into())
            }
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.should_uninitialize {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

struct Pidl(*mut ITEMIDLIST);

impl Pidl {
    fn try_new(path: impl AsRef<Path>) -> eyre::Result<Self> {
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        let path = WideString::from_path(path.as_ref());
        unsafe {
            SHParseDisplayName(PCWSTR(path.as_ptr()), None, &mut pidl, 0, None)
                .wrap_err_with(|| format!("Failed to parse shell path: {}", path.display()))?;
        }
        Ok(Self(pidl))
    }

    fn as_ptr(&self) -> *mut ITEMIDLIST {
        self.0
    }

    fn child_pidl(&self) -> eyre::Result<BorrowedPidl<'_>> {
        let mut child_pidl_raw: *mut ITEMIDLIST = std::ptr::null_mut();
        let _parent_folder: IShellFolder =
            unsafe { SHBindToParent(self.0, Some(&mut child_pidl_raw))? };
        Ok(BorrowedPidl {
            ptr: child_pidl_raw as *const _,
            _lifetime: std::marker::PhantomData,
        })
    }
}

impl Drop for Pidl {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CoTaskMemFree(Some(self.0.cast()));
            }
        }
    }
}

#[derive(Clone, Copy)]
struct BorrowedPidl<'a> {
    ptr: *const ITEMIDLIST,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl BorrowedPidl<'_> {
    fn as_ptr(&self) -> *const ITEMIDLIST {
        self.ptr
    }
}

struct WideString {
    value: Vec<u16>,
    display: String,
}

impl WideString {
    fn from_path(path: &Path) -> Self {
        Self {
            value: path.as_os_str().encode_wide().chain(once(0)).collect(),
            display: path.display().to_string(),
        }
    }

    fn as_ptr(&self) -> *const u16 {
        self.value.as_ptr()
    }

    fn display(&self) -> &str {
        &self.display
    }
}
