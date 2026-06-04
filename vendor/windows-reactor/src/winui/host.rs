use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::Foundation::{HWND as Win32Hwnd, POINTL};
use windows::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, IDataObject, TYMED_HGLOBAL};
use windows::Win32::System::Ole::{
    CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE, IDropTarget, IDropTarget_Impl,
    OleInitialize, RegisterDragDrop, ReleaseStgMedium, RevokeDragDrop,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows_core::Interface;

use super::*;
use crate::bindings::*;

type HDROP = *mut core::ffi::c_void;
type LRESULT = isize;
type SubclassProc =
    Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM, usize, usize) -> LRESULT>;

const WM_DROPFILES: u32 = 0x0233;
const WM_CLOSE: u32 = 0x0010;
const DROP_QUERY_FILE_COUNT: u32 = u32::MAX;
const FILE_DROP_SUBCLASS_ID: usize = 0x434d_6472_6f70;

windows_core::link!("comctl32.dll" "system" fn DefSubclassProc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT);
windows_core::link!("comctl32.dll" "system" fn SetWindowSubclass(hwnd: HWND, pfnsubclass: SubclassProc, uidsubclass: usize, dwrefdata: usize) -> windows_core::BOOL);
windows_core::link!("shell32.dll" "system" fn DragAcceptFiles(hwnd: HWND, faccept: windows_core::BOOL));
windows_core::link!("shell32.dll" "system" fn DragFinish(hdrop: HDROP));
windows_core::link!("shell32.dll" "system" fn DragQueryFileW(hdrop: HDROP, ifile: u32, lpszfile: *mut u16, cch: u32) -> u32);

thread_local! {
    static ROOT_FRAMEWORK_ELEMENT: RefCell<Option<FrameworkElement>> = const { RefCell::new(None) };
    static ROOT_WINDOW: RefCell<Option<Window>> = const { RefCell::new(None) };
    /// Queued theme; applied once `ROOT_FRAMEWORK_ELEMENT` is available.
    static PENDING_THEME: Cell<Option<ElementTheme>> = const { Cell::new(None) };
    static CURRENT_REQUESTED_THEME: Cell<ElementTheme> = const { Cell::new(ElementTheme::Default) };
    /// TitleBar height option requested before `ROOT_WINDOW` was set. Applied once
    /// the window becomes available in `post_render`.
    static PENDING_TALL: Cell<Option<bool>> = const { Cell::new(None) };
    static FILE_DROP_HANDLER: RefCell<Option<crate::core::callback::Callback<Vec<String>>>> = const { RefCell::new(None) };
    static FILE_DROP_HOVER_HANDLER: RefCell<Option<crate::core::callback::Callback<bool>>> = const { RefCell::new(None) };
    static FILE_DROP_HWND: Cell<HWND> = const { Cell::new(core::ptr::null_mut()) };
    static FILE_DROP_SUBCLASS_INSTALLED: Cell<bool> = const { Cell::new(false) };
    static FILE_DROP_TARGET: RefCell<Option<IDropTarget>> = const { RefCell::new(None) };
    static FILE_DROP_TARGET_HWND: Cell<HWND> = const { Cell::new(core::ptr::null_mut()) };
    static FILE_DROP_ACCEPTING_DRAG: Cell<bool> = const { Cell::new(false) };
    static FILE_DROP_HOVERING: Cell<bool> = const { Cell::new(false) };
    static OLE_INITIALIZED_FOR_DROP: Cell<bool> = const { Cell::new(false) };
}

struct FileDropTarget;

windows_core::implement_decl! {
    impl FileDropTarget as FileDropTarget_Impl: [IDropTarget]
}

impl IDropTarget_Impl for FileDropTarget_Impl {
    fn DragEnter(
        &self,
        pdataobj: windows_core::Ref<IDataObject>,
        _grfkeystate: MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> windows_core::Result<()> {
        let accepts = file_drop_handler_present()
            && pdataobj
                .as_ref()
                .is_some_and(|data_object| unsafe { data_object_has_hdrop(data_object) });
        FILE_DROP_ACCEPTING_DRAG.with(|cell| cell.set(accepts));
        set_drop_effect(
            pdweffect,
            if accepts {
                DROPEFFECT_COPY
            } else {
                DROPEFFECT_NONE
            },
        );
        set_file_drop_hovering(accepts);
        Ok(())
    }

    fn DragOver(
        &self,
        _grfkeystate: MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> windows_core::Result<()> {
        let accepts = FILE_DROP_ACCEPTING_DRAG.with(Cell::get) && file_drop_handler_present();
        set_drop_effect(
            pdweffect,
            if accepts {
                DROPEFFECT_COPY
            } else {
                DROPEFFECT_NONE
            },
        );
        set_file_drop_hovering(accepts);
        Ok(())
    }

    fn DragLeave(&self) -> windows_core::Result<()> {
        FILE_DROP_ACCEPTING_DRAG.with(|cell| cell.set(false));
        set_file_drop_hovering(false);
        Ok(())
    }

    fn Drop(
        &self,
        pdataobj: windows_core::Ref<IDataObject>,
        _grfkeystate: MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> windows_core::Result<()> {
        FILE_DROP_ACCEPTING_DRAG.with(|cell| cell.set(false));
        set_file_drop_hovering(false);

        let paths = if file_drop_handler_present() {
            pdataobj
                .as_ref()
                .map(|data_object| unsafe { paths_from_data_object(data_object) })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let accepts = !paths.is_empty();
        set_drop_effect(
            pdweffect,
            if accepts {
                DROPEFFECT_COPY
            } else {
                DROPEFFECT_NONE
            },
        );

        if accepts {
            let handler = FILE_DROP_HANDLER.with(|cell| cell.borrow().clone());
            if let Some(handler) = handler {
                handler.invoke(paths);
            }
        }

        Ok(())
    }
}

/// Requested application theme, matching `Microsoft.UI.Xaml.ElementTheme`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RequestedTheme {
    /// Use the system default (inherits from OS setting).
    Default,
    /// Force light theme.
    Light,
    /// Force dark theme.
    Dark,
}

/// Set the application theme. Queued if the root element isn't attached yet.
pub fn set_requested_theme(theme: RequestedTheme) {
    let element_theme = match theme {
        RequestedTheme::Light => ElementTheme::Light,
        RequestedTheme::Dark => ElementTheme::Dark,
        _ => ElementTheme::Default,
    };
    CURRENT_REQUESTED_THEME.with(|cell| cell.set(element_theme));

    ROOT_FRAMEWORK_ELEMENT.with(|cell| {
        if let Some(ife) = cell.borrow().as_ref() {
            let _ = ife.put_RequestedTheme(element_theme);
            update_titlebar_theme();
        } else {
            PENDING_THEME.with(|p| p.set(Some(element_theme)));
        }
    });
}

/// Register a process-local file drop handler for the active Reactor window.
///
/// Passing `None` disables accepting file drops. Paths are delivered on the UI
/// thread as UTF-8 strings from the Win32 `WM_DROPFILES` payload.
pub fn set_window_file_drop_handler(handler: Option<crate::core::callback::Callback<Vec<String>>>) {
    set_window_file_drop_handlers(handler, None);
}

/// Register process-local file drop and hover handlers for the active Reactor window.
///
/// The drop callback receives UTF-8 paths from Explorer-style file/folder drops. The
/// hover callback is invoked when the current drag can be accepted and again when it
/// leaves or is dropped.
pub fn set_window_file_drop_handlers(
    handler: Option<crate::core::callback::Callback<Vec<String>>>,
    hover_handler: Option<crate::core::callback::Callback<bool>>,
) {
    FILE_DROP_HANDLER.with(|cell| {
        *cell.borrow_mut() = handler;
    });
    FILE_DROP_HOVER_HANDLER.with(|cell| {
        *cell.borrow_mut() = hover_handler;
    });

    FILE_DROP_HWND.with(|cell| {
        let hwnd = cell.get();
        if !hwnd.is_null() {
            sync_file_drop_acceptance(hwnd);
        }
    });
}

fn sync_file_drop_acceptance(hwnd: HWND) {
    let accepts_files = FILE_DROP_HANDLER.with(|cell| cell.borrow().is_some());
    unsafe {
        DragAcceptFiles(hwnd, accepts_files.into());
    }

    if accepts_files {
        ensure_file_drop_subclass(hwnd);
        ensure_ole_file_drop_target(hwnd);
    } else {
        revoke_ole_file_drop_target();
        FILE_DROP_ACCEPTING_DRAG.with(|cell| cell.set(false));
        set_file_drop_hovering(false);
    }
}

fn ensure_file_drop_subclass(hwnd: HWND) {
    let installed = FILE_DROP_SUBCLASS_INSTALLED.with(Cell::get);
    if installed {
        return;
    }

    let installed = unsafe {
        SetWindowSubclass(
            hwnd,
            Some(file_drop_subclass_proc),
            FILE_DROP_SUBCLASS_ID,
            0,
        )
    }
    .as_bool();
    FILE_DROP_SUBCLASS_INSTALLED.with(|cell| cell.set(installed));
}

unsafe extern "system" fn file_drop_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    _ref_data: usize,
) -> LRESULT {
    if msg == WM_DROPFILES {
        let hdrop = wparam as HDROP;
        let paths = unsafe { paths_from_hdrop(hdrop) };
        unsafe {
            DragFinish(hdrop);
        }

        if !paths.is_empty() {
            let handler = FILE_DROP_HANDLER.with(|cell| cell.borrow().clone());
            if let Some(handler) = handler {
                handler.invoke(paths);
            }
        }

        return 0;
    }

    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

unsafe fn paths_from_hdrop(hdrop: HDROP) -> Vec<String> {
    let count = unsafe { DragQueryFileW(hdrop, DROP_QUERY_FILE_COUNT, core::ptr::null_mut(), 0) };
    let mut paths = Vec::with_capacity(count as usize);
    for index in 0..count {
        let len = unsafe { DragQueryFileW(hdrop, index, core::ptr::null_mut(), 0) };
        if len == 0 {
            continue;
        }

        let mut buffer = vec![0_u16; len as usize + 1];
        let written = unsafe { DragQueryFileW(hdrop, index, buffer.as_mut_ptr(), len + 1) };
        if written == 0 {
            continue;
        }

        paths.push(String::from_utf16_lossy(&buffer[..written as usize]));
    }
    paths
}

fn file_drop_handler_present() -> bool {
    FILE_DROP_HANDLER.with(|cell| cell.borrow().is_some())
}

fn set_drop_effect(pdweffect: *mut DROPEFFECT, effect: DROPEFFECT) {
    if !pdweffect.is_null() {
        unsafe {
            *pdweffect = effect;
        }
    }
}

fn set_file_drop_hovering(hovering: bool) {
    let changed = FILE_DROP_HOVERING.with(|cell| {
        let changed = cell.get() != hovering;
        cell.set(hovering);
        changed
    });
    if changed {
        let handler = FILE_DROP_HOVER_HANDLER.with(|cell| cell.borrow().clone());
        if let Some(handler) = handler {
            handler.invoke(hovering);
        }
    }
}

fn ensure_ole_file_drop_target(hwnd: HWND) {
    let registered_hwnd = FILE_DROP_TARGET_HWND.with(Cell::get);
    if registered_hwnd == hwnd && !registered_hwnd.is_null() {
        return;
    }

    revoke_ole_file_drop_target();

    if !ensure_ole_initialized_for_drop() {
        return;
    }

    let target: IDropTarget = FileDropTarget.into();
    let registered = unsafe { RegisterDragDrop(Win32Hwnd(hwnd), &target) }.is_ok();
    if registered {
        FILE_DROP_TARGET.with(|cell| *cell.borrow_mut() = Some(target));
        FILE_DROP_TARGET_HWND.with(|cell| cell.set(hwnd));
    }
}

fn revoke_ole_file_drop_target() {
    let hwnd = FILE_DROP_TARGET_HWND.with(Cell::get);
    if !hwnd.is_null() {
        let _ = unsafe { RevokeDragDrop(Win32Hwnd(hwnd)) };
    }
    FILE_DROP_TARGET.with(|cell| *cell.borrow_mut() = None);
    FILE_DROP_TARGET_HWND.with(|cell| cell.set(core::ptr::null_mut()));
}

fn ensure_ole_initialized_for_drop() -> bool {
    if OLE_INITIALIZED_FOR_DROP.with(Cell::get) {
        return true;
    }

    let initialized = unsafe { OleInitialize(None) }.is_ok();
    OLE_INITIALIZED_FOR_DROP.with(|cell| cell.set(initialized));
    initialized
}

fn hdrop_format() -> FORMATETC {
    FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: core::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

unsafe fn data_object_has_hdrop(data_object: &IDataObject) -> bool {
    unsafe { data_object.QueryGetData(&hdrop_format()).is_ok() }
}

unsafe fn paths_from_data_object(data_object: &IDataObject) -> Vec<String> {
    let format = hdrop_format();
    let mut medium = match unsafe { data_object.GetData(&format) } {
        Ok(medium) => medium,
        Err(_) => return Vec::new(),
    };

    let paths = if medium.tymed == TYMED_HGLOBAL.0 as u32 {
        unsafe { paths_from_hdrop(medium.u.hGlobal.0) }
    } else {
        Vec::new()
    };

    unsafe {
        ReleaseStgMedium(&mut medium);
    }

    paths
}

fn update_titlebar_theme() {
    ROOT_FRAMEWORK_ELEMENT.with(|cell| {
        if let Some(ife) = cell.borrow().as_ref()
            && let Ok(theme) = ife.get_ActualTheme()
        {
            let titlebar_theme = match theme {
                ElementTheme::Dark => TitleBarTheme::Dark,
                ElementTheme::Light => TitleBarTheme::Light,
                _ => TitleBarTheme::UseDefaultAppMode,
            };

            let _ = ROOT_WINDOW.with(|wcell| -> Option<()> {
                let window = wcell.borrow();
                let window_2 = window.as_ref()?.cast::<IWindow2>().ok()?;
                let app_window = window_2.get_AppWindow().ok()?;
                let titlebar = app_window
                    .get_TitleBar()
                    .ok()?
                    .cast::<IAppWindowTitleBar3>()
                    .ok()?;
                titlebar.put_PreferredTheme(titlebar_theme).ok()
            });
        }
    });
}

pub(crate) fn set_titlebar_height(tall: bool) {
    let applied = ROOT_WINDOW.with(|wcell| -> Option<()> {
        let window = wcell.borrow();
        let window_2 = window.as_ref()?.cast::<IWindow2>().ok()?;
        let app_window = window_2.get_AppWindow().ok()?;
        let titlebar = app_window
            .get_TitleBar()
            .ok()?
            .cast::<IAppWindowTitleBar2>()
            .ok()?;
        let option = if tall {
            TitleBarHeightOption::Tall
        } else {
            TitleBarHeightOption::Standard
        };
        titlebar.put_PreferredHeightOption(option).ok()
    });
    if applied.is_none() {
        PENDING_TALL.with(|p| p.set(Some(tall)));
    }
}

/// Apply or remove the window backdrop material at runtime.
pub fn set_backdrop(backdrop: Option<Backdrop>) {
    ROOT_WINDOW.with(|cell| {
        if let Some(window) = cell.borrow().as_ref() {
            if let Some(b) = backdrop {
                let _ = b.apply_to(window);
            } else {
                if let Ok(w2) = window.cast::<IWindow2>() {
                    let _ = w2.put_SystemBackdrop(None);
                }
            }
        }
    });
}

/// Close the active Reactor root window, if one is currently attached.
pub fn close_root_window() {
    ROOT_WINDOW.with(|cell| {
        let window_slot = cell.borrow();
        let Some(window) = window_slot.as_ref() else {
            return;
        };

        if let Ok(native) = window.cast::<IWindowNative>() {
            let mut hwnd: HWND = HWND::default();
            if unsafe { native.get_WindowHandle(&mut hwnd) }.is_ok() && !hwnd.is_null() {
                unsafe {
                    let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM::default(), LPARAM::default());
                }
            }
        }
    });
}

/// Top-level window presenter (`AppWindowPresenterKind`).
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum PresenterKind {
    /// Platform default (overlapping window with a title bar).
    #[default]
    Default,
    /// Frameless, fills the active monitor.
    FullScreen,
    /// Floating "picture-in-picture" style overlay.
    CompactOverlay,
}

impl PresenterKind {
    fn to_native(self) -> Option<AppWindowPresenterKind> {
        match self {
            PresenterKind::Default => None,
            PresenterKind::FullScreen => Some(AppWindowPresenterKind::FullScreen),
            PresenterKind::CompactOverlay => Some(AppWindowPresenterKind::CompactOverlay),
        }
    }
}

/// Window backdrop material applied behind the app content.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Backdrop {
    Mica,
    MicaAlt,
    Acrylic,
}

impl Backdrop {
    /// Apply this backdrop material to an existing WinUI window.
    ///
    /// This is useful for manual window setup in [`crate::app::App::run_custom`]
    /// or other custom hosts that do not go through [`ReactorHost`].
    pub fn apply_to(self, window: &impl Interface) -> windows_core::Result<()> {
        let system_backdrop: SystemBackdrop = match self {
            Backdrop::Mica => MicaBackdrop::new()?.cast()?,
            Backdrop::MicaAlt => {
                let mica = MicaBackdrop::new()?;
                mica.put_Kind(MicaKind::BaseAlt)?;
                mica.cast()?
            }
            Backdrop::Acrylic => DesktopAcrylicBackdrop::new()?.cast()?,
        };
        window
            .cast::<IWindow2>()?
            .put_SystemBackdrop(&system_backdrop)
    }
}

/// WinUI-bound [`RenderHost`] hosting a single root [`Component`] inside
/// a `Microsoft.UI.Xaml.Window`.
pub struct ReactorHost {
    render_host: RenderHost<WinUIBackend, WinUIDispatcher>,
    window: Window,
    presenter: Cell<PresenterKind>,
    backdrop: Cell<Option<Backdrop>>,
}

impl ReactorHost {
    pub fn new(title: impl AsRef<str>, root: Box<dyn Component>) -> windows_core::Result<Self> {
        Self::new_with(title, root, |_| {})
    }

    pub fn new_with<F>(
        title: impl AsRef<str>,
        root: Box<dyn Component>,
        configure: F,
    ) -> windows_core::Result<Self>
    where
        F: FnOnce(&mut crate::core::reconciler::Reconciler<WinUIBackend>),
    {
        Self::new_with_window_options(title, None, InnerConstraints::default(), root, configure)
    }

    pub fn new_with_window_options<F>(
        title: impl AsRef<str>,
        size: Option<crate::core::Size>,
        constraints: InnerConstraints,
        root: Box<dyn Component>,
        configure: F,
    ) -> windows_core::Result<Self>
    where
        F: FnOnce(&mut crate::core::reconciler::Reconciler<WinUIBackend>),
    {
        let (window, resolved_dip_size, initial_dpi) = create_window(title, size, constraints)?;
        let dispatcher = WinUIDispatcher::for_current_thread()?;
        let marshaller = dispatcher.marshaller();
        let render_host = RenderHost::new(WinUIBackend::new(), root, dispatcher);
        render_host.set_marshaller(Some(marshaller));
        render_host.set_inner_size(resolved_dip_size);
        render_host.set_dpi(initial_dpi);
        render_host.with_reconciler_mut(configure);

        let attach_for_post_render = AttachState {
            window: window.clone(),
            render_host: render_host.clone_inner(),
        };
        let last_attached: Rc<Cell<Option<ControlId>>> = Rc::new(Cell::new(None));
        let last_attached_for_hook = Rc::clone(&last_attached);
        let subscribed = Rc::new(Cell::new(false));
        render_host.set_post_render(move |new_id| {
            if last_attached_for_hook.get() == new_id {
                return;
            }
            let state = &attach_for_post_render;
            match new_id {
                Some(rid) => {
                    if let Some(ui) = state.render_host.with_backend(|b| b.get_ui_element(rid)) {
                        let ui_element: UIElement = ui.cast().unwrap();
                        let _ = state.window.put_Content(&ui_element);
                        last_attached_for_hook.set(Some(rid));

                        ROOT_WINDOW.with(|cell| *cell.borrow_mut() = Some(state.window.clone()));
                        if let Ok(fe) = ui_element.cast::<FrameworkElement>() {
                            ROOT_FRAMEWORK_ELEMENT
                                .with(|cell| *cell.borrow_mut() = Some(fe.clone()));

                            let requested_theme = PENDING_THEME
                                .with(|p| p.take())
                                .unwrap_or_else(|| CURRENT_REQUESTED_THEME.with(Cell::get));
                            let _ = fe.put_RequestedTheme(requested_theme);
                            subscribe_actual_theme_changed(&fe, state.render_host.clone_inner());
                            update_titlebar_theme();

                            if !subscribed.get() {
                                subscribed.set(true);
                                subscribe_size_and_dpi(
                                    &fe,
                                    state.render_host.clone_inner(),
                                    state.window.clone(),
                                    constraints,
                                );
                            }
                        }

                        // Wire TitleBar to window on every root change (mirrors C# mount behavior).
                        if let Some(tb) = state.render_host.with_backend(|b| b.find_titlebar()) {
                            let _ = state.window.put_ExtendsContentIntoTitleBar(true);
                            if let Ok(tb_ui) = tb.cast::<UIElement>() {
                                let _ = state.window.SetTitleBar(&tb_ui);
                            }
                            // SetPreferredHeightOption is silently ignored unless
                            // ExtendsContentIntoTitleBar is already true.
                            if let Some(tall) = PENDING_TALL.with(|p| p.take()) {
                                set_titlebar_height(tall);
                            }
                        }
                    }
                }
                None => {
                    last_attached_for_hook.set(None);
                }
            }
        });

        render_host.kick();

        Ok(Self {
            render_host,
            window,
            presenter: Cell::new(PresenterKind::Default),
            backdrop: Cell::new(None),
        })
    }

    /// Set the window presenter (full-screen / compact overlay / default).
    /// Must be called before [`Self::activate`].
    pub fn set_presenter(&self, kind: PresenterKind) {
        self.presenter.set(kind);
    }

    /// Set the window backdrop material (Mica, Mica Alt, or Acrylic).
    /// Must be called before [`Self::activate`].
    pub fn set_backdrop(&self, backdrop: Backdrop) {
        self.backdrop.set(Some(backdrop));
    }

    pub fn activate(&self) -> windows_core::Result<()> {
        let presenter = self.presenter.get();
        let backdrop = self.backdrop.get();
        let window = self.window.clone();
        let handler = DispatcherQueueHandler::new(move || {
            let _ = (|| -> windows_core::Result<()> {
                let mut hwnd: HWND = HWND::default();
                if let Ok(native) = window.cast::<IWindowNative>() {
                    let _ = unsafe { native.get_WindowHandle(&mut hwnd) };
                }

                if let Some(native_kind) = presenter.to_native()
                    && let Ok(app_window) = window.cast::<IWindow2>()?.get_AppWindow()
                {
                    let _ = app_window.SetPresenterByKind(native_kind);
                }
                if let Some(bd) = backdrop
                    && let Err(err) = bd.apply_to(&window)
                {
                    eprintln!("windows-reactor: backdrop failed: {err}");
                }
                let _ = window.Activate();

                // Clear the OS-supplied AppStarting cursor by posting a synthetic
                // WM_SETCURSOR; otherwise the spinner persists until the first
                // mouse move. PostMessageW (not SendMessageW) avoids flicker.
                if !hwnd.is_null() {
                    FILE_DROP_HWND.with(|cell| cell.set(hwnd));
                    sync_file_drop_acceptance(hwnd);

                    let lparam: LPARAM =
                        (((WM_MOUSEMOVE) << 16) | (HTCLIENT & 0xFFFF)) as i32 as LPARAM;
                    unsafe {
                        let _ = PostMessageW(hwnd, WM_SETCURSOR, hwnd as WPARAM, lparam);
                    }
                }
                Ok(())
            })();
        });
        let queue = DispatcherQueue::GetForCurrentThread()?;
        queue.TryEnqueueWithPriority(DispatcherQueuePriority::High, &handler)?;
        Ok(())
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn stats(&self) -> RenderStats {
        self.render_host.stats()
    }

    pub fn set_render_complete<F>(&self, f: F)
    where
        F: Fn(f64, f64, f64) + 'static,
    {
        self.render_host.set_render_complete(f);
    }
}

fn get_default_display_size(hwnd: HWND, dpi: u32) -> crate::core::Size {
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut monitor_info_ex = MONITORINFOEXW {
            monitorInfo: MONITORINFO {
                cbSize: core::mem::size_of::<MONITORINFOEXW>() as u32,
                ..MONITORINFO::default()
            },
            ..MONITORINFOEXW::default()
        };
        if GetMonitorInfoW(monitor, &mut monitor_info_ex.monitorInfo).as_bool() {
            let work = monitor_info_ex.monitorInfo.rcWork;
            let work_width = work.right.saturating_sub(work.left);
            let work_height = work.bottom.saturating_sub(work.top);
            let scale = dpi as f64 / 96.0;
            crate::core::Size {
                width: work_width as f64 / scale / 2.0,
                height: work_height as f64 / scale / 2.0,
            }
        } else {
            crate::core::Size::default()
        }
    }
}

fn center_window_on_display(
    hwnd: HWND,
    client_width_px: i32,
    client_height_px: i32,
    nc_width_px: i32,
    nc_height_px: i32,
) {
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut monitor_info_ex = MONITORINFOEXW {
            monitorInfo: MONITORINFO {
                cbSize: core::mem::size_of::<MONITORINFOEXW>() as u32,
                ..MONITORINFO::default()
            },
            ..MONITORINFOEXW::default()
        };
        if !GetMonitorInfoW(monitor, &mut monitor_info_ex.monitorInfo).as_bool() {
            return;
        }
        let work = monitor_info_ex.monitorInfo.rcWork;
        let work_width = work.right.saturating_sub(work.left);
        let work_height = work.bottom.saturating_sub(work.top);

        let outer_width = client_width_px.saturating_add(nc_width_px);
        let outer_height = client_height_px.saturating_add(nc_height_px);
        let x = work.left + (work_width.saturating_sub(outer_width)) / 2;
        let y = work.top + (work_height.saturating_sub(outer_height)) / 2;
        let _ = SetWindowPos(
            hwnd,
            HWND::default(),
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

fn subscribe_size_and_dpi(
    fe: &FrameworkElement,
    render_host: RenderHost<WinUIBackend, WinUIDispatcher>,
    window: Window,
    constraints: InnerConstraints,
) {
    let mut hwnd: HWND = HWND::default();
    if let Ok(native) = window.cast::<IWindowNative>() {
        let _ = unsafe { native.get_WindowHandle(&mut hwnd) };
    }

    let _ = fe
        .add_SizeChanged(move |_sender, args| {
            let size = args.unwrap().get_NewSize().unwrap();
            let new_dpi = unsafe { GetDpiForWindow(hwnd) };
            if new_dpi > 0 {
                render_host.set_dpi(new_dpi);
            }
            render_host.set_inner_size(crate::core::Size {
                width: size.Width as f64,
                height: size.Height as f64,
            });
            let _ = apply_constraints_for_window(&window, render_host.dpi(), &constraints);
        })
        .ok()
        .map(|r| r.into_token());
}

fn create_window(
    title: impl AsRef<str>,
    size: Option<crate::core::Size>,
    constraints: InnerConstraints,
) -> Result<(Window, crate::core::Size, u32), windows_core::Error> {
    let window = Window::new()?;

    let mut hwnd = HWND::default();
    unsafe {
        window
            .cast::<IWindowNative>()?
            .get_WindowHandle(&mut hwnd)?;
    }
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let dpi = if dpi == 0 { 96 } else { dpi };

    window.put_Title(title.as_ref())?;

    let dip_size = match size {
        Some(s) => s,
        None => get_default_display_size(hwnd, dpi),
    };

    let dip_to_px = |dips: f64| (dips * dpi as f64 / 96.0).round() as i32;

    let window_2 = window.cast::<IWindow2>()?;
    let app_window = window_2.get_AppWindow()?;
    let app_window_2 = app_window.cast::<IAppWindow2>()?;
    app_window_2.ResizeClient(SizeInt32 {
        Width: dip_to_px(dip_size.width),
        Height: dip_to_px(dip_size.height),
    })?;

    app_window.SetPresenterByKind(AppWindowPresenterKind::Overlapped)?;
    set_requested_theme(RequestedTheme::Default);

    let outer_size = app_window.get_Size()?;
    let inner_size = app_window_2.get_ClientSize()?;
    let nc_width_px = outer_size.Width.saturating_sub(inner_size.Width);
    let nc_height_px = outer_size.Height.saturating_sub(inner_size.Height);

    let overlapped = app_window
        .get_Presenter()?
        .cast::<IOverlappedPresenter3>()?;
    if let Some(min_w) = constraints.min_width {
        overlapped.put_PreferredMinimumWidth(Some(dip_to_px(min_w).saturating_add(nc_width_px)))?;
    }
    if let Some(min_h) = constraints.min_height {
        overlapped
            .put_PreferredMinimumHeight(Some(dip_to_px(min_h).saturating_add(nc_height_px)))?;
    }
    if let Some(max_w) = constraints.max_width {
        overlapped.put_PreferredMaximumWidth(Some(dip_to_px(max_w).saturating_add(nc_width_px)))?;
    }
    if let Some(max_h) = constraints.max_height {
        overlapped
            .put_PreferredMaximumHeight(Some(dip_to_px(max_h).saturating_add(nc_height_px)))?;
    }

    let actual_client_px = app_window_2.get_ClientSize()?;
    let actual_dip_size = crate::core::Size {
        width: actual_client_px.Width as f64 * 96.0 / dpi as f64,
        height: actual_client_px.Height as f64 * 96.0 / dpi as f64,
    };

    center_window_on_display(
        hwnd,
        actual_client_px.Width,
        actual_client_px.Height,
        nc_width_px,
        nc_height_px,
    );

    Ok((window, actual_dip_size, dpi))
}

/// Re-apply DIP `constraints` to the window's `OverlappedPresenter`,
/// re-measuring the non-client offset at current DPI.
fn apply_constraints_for_window(
    window: &Window,
    dpi: u32,
    constraints: &InnerConstraints,
) -> windows_core::Result<()> {
    let dip_scale = dpi as f64 / 96.0;
    let dip_to_px = |dips: f64| (dips * dip_scale).round() as i32;

    let app_window = window.cast::<IWindow2>()?.get_AppWindow()?;
    let app_window_2 = app_window.cast::<IAppWindow2>()?;

    let outer_size = app_window.get_Size()?;
    let inner_size = app_window_2.get_ClientSize()?;
    let nc_width_px = outer_size.Width.saturating_sub(inner_size.Width);
    let nc_height_px = outer_size.Height.saturating_sub(inner_size.Height);

    let presenter = app_window
        .get_Presenter()?
        .cast::<IOverlappedPresenter3>()?;

    if let Some(min_w) = constraints.min_width {
        presenter.put_PreferredMinimumWidth(Some(dip_to_px(min_w).saturating_add(nc_width_px)))?;
    }
    if let Some(min_h) = constraints.min_height {
        presenter
            .put_PreferredMinimumHeight(Some(dip_to_px(min_h).saturating_add(nc_height_px)))?;
    }
    if let Some(max_w) = constraints.max_width {
        presenter.put_PreferredMaximumWidth(Some(dip_to_px(max_w).saturating_add(nc_width_px)))?;
    }
    if let Some(max_h) = constraints.max_height {
        presenter
            .put_PreferredMaximumHeight(Some(dip_to_px(max_h).saturating_add(nc_height_px)))?;
    }
    Ok(())
}

impl<B: Backend + 'static, D: Dispatcher + 'static> RenderHost<B, D> {
    pub fn with_backend<R>(&self, f: impl FnOnce(&B) -> R) -> R {
        self.with_reconciler(|r| f(&r.backend))
    }
}

fn subscribe_actual_theme_changed(
    fe: &FrameworkElement,
    render_host: RenderHost<WinUIBackend, WinUIDispatcher>,
) {
    update_color_scheme_from(fe);

    let _ = fe
        .add_ActualThemeChanged(move |sender, _| {
            if let Some(fe) = sender.as_ref() {
                update_color_scheme_from(fe);
                update_titlebar_theme();
            }
            render_host.with_reconciler_mut(|r| r.notify_theme_changed());
            render_host.request_render();
        })
        .ok()
        .map(|r| r.into_token());
}

fn update_color_scheme_from(fe: &FrameworkElement) {
    if let Ok(theme) = fe.get_ActualTheme() {
        let scheme = match theme {
            ElementTheme::Dark => crate::core::theme::ColorScheme::Dark,
            _ => crate::core::theme::ColorScheme::Light,
        };
        crate::core::theme::set_current_color_scheme(scheme);
    }
}

struct AttachState {
    window: Window,
    render_host: RenderHost<WinUIBackend, WinUIDispatcher>,
}
