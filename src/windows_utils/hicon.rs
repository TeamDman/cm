use eyre::bail;
use tracing::debug;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::UI::WindowsAndMessaging::HICON;
use windows::Win32::UI::WindowsAndMessaging::LoadIconW;
use windows::core::PCWSTR;
use windows::core::Param;

pub fn get_icon_from_current_module(
    icon_name: impl Param<PCWSTR> + std::fmt::Debug,
) -> eyre::Result<HICON> {
    let handle = unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(None)? };
    debug!(?handle, "trying to load embedded icon from current module");
    let icon = unsafe { LoadIconW(Some(HINSTANCE(handle.0)), icon_name) };
    match icon {
        Ok(icon) => Ok(icon),
        Err(err) => bail!("Failed to load embedded icon from current module: {err}"),
    }
}
