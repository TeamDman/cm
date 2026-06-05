use eyre::eyre;
#[cfg(windows)]
use std::ffi::OsStr;
use std::fs;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::time::Instant;
#[cfg(windows)]
use windows::Win32::Foundation::HANDLE;
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
use windows::Win32::System::DataExchange::CloseClipboard;
#[cfg(windows)]
use windows::Win32::System::DataExchange::EmptyClipboard;
#[cfg(windows)]
use windows::Win32::System::DataExchange::OpenClipboard;
#[cfg(windows)]
use windows::Win32::System::DataExchange::SetClipboardData;
#[cfg(windows)]
use windows::Win32::System::Memory::GMEM_MOVEABLE;
#[cfg(windows)]
use windows::Win32::System::Memory::GlobalAlloc;
#[cfg(windows)]
use windows::Win32::System::Memory::GlobalLock;
#[cfg(windows)]
use windows::Win32::System::Memory::GlobalUnlock;
#[cfg(windows)]
use windows::Win32::UI::Shell::FOS_ALLOWMULTISELECT;
#[cfg(windows)]
use windows::Win32::UI::Shell::FOS_FORCEFILESYSTEM;
#[cfg(windows)]
use windows::Win32::UI::Shell::FOS_PICKFOLDERS;
#[cfg(windows)]
use windows::Win32::UI::Shell::FileOpenDialog;
#[cfg(windows)]
use windows::Win32::UI::Shell::IFileOpenDialog;
#[cfg(windows)]
use windows::Win32::UI::Shell::SIGDN_FILESYSPATH;
#[expect(
    clippy::wildcard_imports,
    reason = "reactor UI DSL is built around a broad prelude"
)]
use windows_reactor::*;

const PICK_FILES_LABEL: &str = "Pick files";
const PICK_FOLDERS_LABEL: &str = "Pick folders";
#[cfg(windows)]
const CF_UNICODETEXT_FORMAT: u32 = 13;

#[cfg(windows)]
struct ClipboardGuard;

#[cfg(windows)]
impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InitialSurface {
    MainMenu,
    Studio,
    ProductSearch,
}

impl InitialSurface {
    fn into_mode(self) -> AppMode {
        match self {
            Self::MainMenu => AppMode::MainMenu,
            Self::Studio => AppMode::Studio,
            Self::ProductSearch => AppMode::ProductSearch,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppMode {
    MainMenu,
    Studio,
    ProductSearch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WizardStep {
    PickInputPaths,
    Output,
    Processing,
    Naming,
    Review,
}

impl WizardStep {
    fn tag(self) -> &'static str {
        match self {
            Self::PickInputPaths => "pick-input-paths",
            Self::Output => "output",
            Self::Processing => "processing",
            Self::Naming => "naming",
            Self::Review => "review",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::PickInputPaths => "Pick input paths",
            Self::Output => "Output",
            Self::Processing => "Processing",
            Self::Naming => "Naming",
            Self::Review => "Review",
        }
    }

    fn from_tag(tag: &str) -> Self {
        match tag {
            "output" => Self::Output,
            "processing" => Self::Processing,
            "naming" => Self::Naming,
            "review" => Self::Review,
            _ => Self::PickInputPaths,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct StudioState {
    active_step: WizardStep,
    selected_root_paths: Vec<PathBuf>,
    scan_generation: u64,
}

impl Default for StudioState {
    fn default() -> Self {
        Self {
            active_step: WizardStep::PickInputPaths,
            selected_root_paths: Vec::new(),
            scan_generation: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScanStatus {
    Empty,
    Loading,
    Ready,
    ReadyWithIssues,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RootScanPhase {
    NotStarted,
    InProgress,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
struct InputScanState {
    generation: u64,
    status: ScanStatus,
    roots: Vec<InputRootRow>,
    transitive_entries: Vec<TransitiveInputRow>,
    issues: Vec<ScanIssue>,
}

impl InputScanState {
    fn empty(generation: u64) -> Self {
        Self {
            generation,
            status: ScanStatus::Empty,
            roots: Vec::new(),
            transitive_entries: Vec::new(),
            issues: Vec::new(),
        }
    }

    fn loading(generation: u64, roots: &[PathBuf]) -> Self {
        Self {
            generation,
            status: ScanStatus::Loading,
            roots: roots
                .iter()
                .cloned()
                .map(InputRootRow::not_started)
                .collect(),
            transitive_entries: Vec::new(),
            issues: Vec::new(),
        }
    }

    fn finalize_status(mut self) -> Self {
        self.status = if self.roots.is_empty() {
            ScanStatus::Empty
        } else if self.roots.iter().any(|root| {
            matches!(
                root.phase,
                RootScanPhase::NotStarted | RootScanPhase::InProgress
            )
        }) {
            ScanStatus::Loading
        } else if self.issues.is_empty() {
            ScanStatus::Ready
        } else {
            ScanStatus::ReadyWithIssues
        };
        self
    }
}

impl Default for InputScanState {
    fn default() -> Self {
        Self::empty(0)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct InputRootRow {
    path: PathBuf,
    phase: RootScanPhase,
    started_at: Option<Instant>,
    discovered_count: usize,
    issue: Option<String>,
}

impl InputRootRow {
    fn not_started(path: PathBuf) -> Self {
        Self {
            path,
            phase: RootScanPhase::NotStarted,
            started_at: None,
            discovered_count: 0,
            issue: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TransitiveInputRow {
    path: PathBuf,
    source_root: PathBuf,
}

#[derive(Clone, PartialEq)]
struct AppModeProps {
    set_mode: SetState<AppMode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScanIssue {
    root_path: PathBuf,
    message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RootScanResult {
    entries: Vec<TransitiveInputRow>,
    issue: Option<String>,
}

pub(crate) fn run(initial_surface: InitialSurface) -> eyre::Result<()> {
    App::new()
        .title("CM Reactor Shell")
        .backdrop(Backdrop::Mica)
        .eager_templated_realization(true)
        .render(move |cx| app(cx, initial_surface.into_mode()))
        .map_err(|error| eyre!("{error}"))
}

fn app(cx: &mut RenderCx, initial_mode: AppMode) -> Element {
    crate::windows_cli::console::register_reactor_marshaller(&cx.use_ui_marshaller());

    if crate::windows_cli::console::take_close_requested() {
        close_active_reactor_window();
    }

    let (mode, set_mode) = cx.use_state(initial_mode);
    let props = AppModeProps { set_mode };

    match mode {
        AppMode::MainMenu => component(main_menu, props),
        AppMode::Studio => component(studio, props),
        AppMode::ProductSearch => component(product_search, props),
    }
}

#[cfg(windows)]
fn close_active_reactor_window() {
    close_root_window();
}

#[cfg(not(windows))]
fn close_active_reactor_window() {}

fn main_menu(props: &AppModeProps, cx: &mut RenderCx) -> Element {
    set_window_file_drop_handler(None);
    let is_dark = matches!(cx.use_color_scheme(), ColorScheme::Dark);
    let (launch_status, set_launch_status) = cx.use_state(Option::<String>::None);
    let set_mode = props.set_mode.clone();

    let open_reactor_studio = {
        let set_mode = set_mode.clone();
        move || set_mode.call(AppMode::Studio)
    };
    let open_reactor_product_search = {
        let set_mode = set_mode.clone();
        move || set_mode.call(AppMode::ProductSearch)
    };
    let open_egui_studio = {
        let set_launch_status = set_launch_status.clone();
        move || set_launch_status.call(Some(launch_gui_mode_status("egui-studio")))
    };
    let open_egui_product_search =
        move || set_launch_status.call(Some(launch_gui_mode_status("egui-product-search")));

    let title_bar = TitleBar::new("CM Reactor Shell")
        .footer(theme_toggle_button(is_dark))
        .tall(true);
    let action_grid: Element = grid((
        launch_button(
            "egui Studio",
            "Open the tiled egui studio surface.",
            SymbolGlyph::Edit,
            open_egui_studio,
        )
        .grid_row(0)
        .grid_column(0)
        .automation_id("main-menu-egui-studio"),
        launch_button(
            "Reactor Studio",
            "Open the guided Reactor wizard.",
            SymbolGlyph::Forward,
            open_reactor_studio,
        )
        .grid_row(0)
        .grid_column(1)
        .automation_id("main-menu-reactor-studio"),
        launch_button(
            "egui Product Search",
            "Open product search in the egui surface.",
            SymbolGlyph::Find,
            open_egui_product_search,
        )
        .grid_row(1)
        .grid_column(0)
        .automation_id("main-menu-egui-product-search"),
        launch_button(
            "Reactor Product Search",
            "Search product metadata inside the Reactor surface.",
            SymbolGlyph::Find,
            open_reactor_product_search,
        )
        .grid_row(1)
        .grid_column(1)
        .automation_id("main-menu-reactor-product-search"),
    ))
    .rows([GridLength::Auto, GridLength::Auto])
    .columns([GridLength::Star(1.0), GridLength::Star(1.0)])
    .row_spacing(8.0)
    .column_spacing(8.0)
    .max_width(920.0)
    .into();

    let mut content_children = vec![
        vstack((
            text_block("CM").font_size(28.0).bold(),
            text_block(
                "Choose a surface. Reactor keeps the guided wizard, and egui keeps the tile-based studio.",
            )
            .foreground(ThemeRef::SecondaryText)
            .max_width(680.0)
            .wrap(),
        ))
        .spacing(8.0)
        .into(),
        vstack((text_block("Start").font_size(14.0).semibold(), action_grid))
            .spacing(16.0)
            .into(),
    ];

    if let Some(status) = launch_status {
        content_children.push(
            InfoBar::new("Launch status")
                .message(status)
                .informational()
                .is_closable(false)
                .max_width(720.0)
                .automation_id("main-menu-launch-status")
                .into(),
        );
    }

    let page: Element = border(vstack(content_children).spacing(32.0))
        .background(ThemeRef::SolidBackground)
        .padding(page_padding())
        .into();

    grid((title_bar.grid_row(0), page.grid_row(1)))
        .rows([GridLength::Auto, GridLength::Star(1.0)])
        .columns([GridLength::Star(1.0)])
        .into()
}

#[expect(
    clippy::too_many_lines,
    reason = "UI layout is easier to maintain as one builder function"
)]
fn studio(props: &AppModeProps, cx: &mut RenderCx) -> Element {
    let is_dark = matches!(cx.use_color_scheme(), ColorScheme::Dark);
    let (wizard, set_wizard) = cx.use_state(StudioState::default());
    let (scan, set_scan) = cx.use_async_state(InputScanState::default());
    let (drop_hovering, set_drop_hovering) = cx.use_state(false);
    let (is_pane_open, set_pane_open) = cx.use_state(true);
    let scan = if scan.generation == wizard.scan_generation {
        scan
    } else {
        InputScanState::loading(wizard.scan_generation, &wizard.selected_root_paths)
    };
    install_drop_handler(
        &wizard,
        set_wizard.clone(),
        set_scan.clone(),
        drop_hovering,
        set_drop_hovering.clone(),
    );

    let menu_items = [
        NavViewItem::new(WizardStep::PickInputPaths.label())
            .tag(WizardStep::PickInputPaths.tag())
            .icon(SymbolGlyph::Add),
        NavViewItem::new(WizardStep::Output.label())
            .tag(WizardStep::Output.tag())
            .icon(SymbolGlyph::Save),
        NavViewItem::new(WizardStep::Processing.label())
            .tag(WizardStep::Processing.tag())
            .icon(SymbolGlyph::Sync),
        NavViewItem::new(WizardStep::Naming.label())
            .tag(WizardStep::Naming.tag())
            .icon(SymbolGlyph::Edit),
        NavViewItem::new(WizardStep::Review.label())
            .tag(WizardStep::Review.tag())
            .icon(SymbolGlyph::Accept),
    ];

    let body = match wizard.active_step {
        WizardStep::PickInputPaths => input_paths_step(
            wizard.clone(),
            set_wizard.clone(),
            scan,
            set_scan,
            drop_hovering,
        ),
        WizardStep::Output => placeholder_step(
            "Output",
            "Output directory, flattening, hierarchy, and overwrite choices will come next.",
            &wizard,
        ),
        WizardStep::Processing => placeholder_step(
            "Processing",
            "Image-processing options and plan-building controls will land here.",
            &wizard,
        ),
        WizardStep::Naming => placeholder_step(
            "Naming",
            "Rename rules, collision behavior, and filename limits will land here.",
            &wizard,
        ),
        WizardStep::Review => placeholder_step(
            "Review",
            "A readable plan preview and final execution controls will land here.",
            &wizard,
        ),
    };

    let on_step_changed = {
        let wizard = wizard.clone();
        move |tag: String| {
            let mut next = wizard.clone();
            next.active_step = WizardStep::from_tag(&tag);
            set_wizard.call(next);
        }
    };

    let navigation: Element = NavigationView::new(menu_items, body)
        .selected_tag(wizard.active_step.tag().to_string())
        .on_selection_changed(on_step_changed)
        .pane_display_mode(NavViewPaneDisplayMode::Left)
        .pane_open(is_pane_open)
        .pane_title("Reactor Studio")
        .settings_visible(false)
        .pane_toggle_button_visible(false)
        .back_button_visible(false)
        .font_family("Segoe UI Variable")
        .automation_id("reactor-studio-navigation")
        .into();
    let go_home = {
        let set_mode = props.set_mode.clone();
        move || set_mode.call(AppMode::MainMenu)
    };
    let title_bar = TitleBar::new("CM Reactor Shell")
        .subtitle("Reactor Studio")
        .pane_toggle_button_visible(true)
        .back_button_visible(true)
        .back_button_enabled(true)
        .on_back_requested(go_home)
        .on_pane_toggle_requested(move || set_pane_open.call(!is_pane_open))
        .footer(theme_toggle_button(is_dark))
        .tall(true);
    let drop_overlay = studio_drop_overlay(drop_hovering);

    grid((title_bar.grid_row(0), navigation.grid_row(1), drop_overlay))
        .rows([GridLength::Auto, GridLength::Star(1.0)])
        .columns([GridLength::Star(1.0)])
        .into()
}

#[expect(
    clippy::too_many_lines,
    reason = "UI layout is easier to maintain as one builder function"
)]
fn product_search(props: &AppModeProps, cx: &mut RenderCx) -> Element {
    set_window_file_drop_handler(None);
    let is_dark = matches!(cx.use_color_scheme(), ColorScheme::Dark);
    let (query, set_query) = cx.use_state(String::new());
    let (sku, set_sku) = cx.use_state(String::new());
    let (only_sku, set_only_sku) = cx.use_state(true);
    let (search_output, set_search_output) =
        cx.use_async_state("Enter a query or SKU to search.".to_string());
    let (launch_status, set_launch_status) =
        cx.use_state("The egui Product Search mode can also be launched directly.".to_string());

    let search_fields: Element = grid((
        text_box(query.clone())
            .header("Search query")
            .placeholder("Album, paper pack, seasonal title...")
            .on_changed(set_query)
            .grid_row(0)
            .grid_column(0),
        text_box(sku.clone())
            .header("SKU")
            .placeholder("Optional product SKU")
            .on_changed(set_sku)
            .grid_row(0)
            .grid_column(1),
    ))
    .columns([GridLength::Star(2.0), GridLength::Star(1.0)])
    .column_spacing(12.0)
    .into();

    let only_sku_toggle: Element = check_box(only_sku)
        .label("Only auto-search when a SKU is found")
        .on_changed(set_only_sku)
        .into();
    let can_search = !query.trim().is_empty() || !sku.trim().is_empty();
    let run_search = {
        let query = query.clone();
        let sku = sku.clone();
        let set_search_output = set_search_output.clone();
        move || {
            set_search_output.call("Searching...".to_string());
            let set_search_output = set_search_output.clone();
            let query = query.clone();
            let sku = sku.clone();
            thread::spawn(move || {
                set_search_output.call(run_cm_search(&query, &sku));
            });
        }
    };
    let launch_egui_product_search =
        move || set_launch_status.call(launch_gui_mode_status("egui-product-search"));
    let search_button: Element = button("Search")
        .accent()
        .icon(SymbolGlyph::Find)
        .enabled(can_search)
        .on_click(run_search)
        .min_width(160.0)
        .into();
    let launch_button: Element = button("Open egui Product Search")
        .on_click(launch_egui_product_search)
        .min_width(220.0)
        .into();
    let search_status: Element = scroll_view(text_block(search_output).font_size(13.0).wrap())
        .height(260.0)
        .into();
    let launch_status: Element = text_block(launch_status).font_size(13.0).wrap().into();
    let go_home = {
        let set_mode = props.set_mode.clone();
        move || set_mode.call(AppMode::MainMenu)
    };
    let title_bar = TitleBar::new("CM Reactor Shell")
        .subtitle("Reactor Product Search")
        .back_button_visible(true)
        .back_button_enabled(true)
        .on_back_requested(go_home)
        .footer(theme_toggle_button(is_dark))
        .tall(true);

    let content = grid((
        page_header(
            "Reactor Product Search",
            "A standalone utility mode for Searchspring-style product lookup.",
        )
        .grid_row(0),
        search_fields.grid_row(1),
        only_sku_toggle.grid_row(2),
        hstack((search_button, launch_button))
            .spacing(8.0)
            .grid_row(3),
        launch_status.grid_row(4),
        border(search_status)
            .border_brush(ThemeRef::CardStroke)
            .border_thickness(Thickness::uniform(1.0))
            .corner_radius(8.0)
            .padding(Thickness::uniform(8.0))
            .max_width(920.0)
            .grid_row(5),
    ))
    .rows([
        GridLength::Auto,
        GridLength::Auto,
        GridLength::Auto,
        GridLength::Auto,
        GridLength::Auto,
        GridLength::Auto,
    ])
    .columns([GridLength::Star(1.0)])
    .row_spacing(16.0)
    .max_width(920.0);

    grid((title_bar.grid_row(0), page_shell(content).grid_row(1)))
        .rows([GridLength::Auto, GridLength::Star(1.0)])
        .columns([GridLength::Star(1.0)])
        .into()
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "state handles are cloned into event callbacks"
)]
fn input_paths_step(
    wizard: StudioState,
    set_wizard: SetState<StudioState>,
    scan: InputScanState,
    set_scan: AsyncSetState<InputScanState>,
    drop_hovering: bool,
) -> Element {
    let has_selected_paths = !wizard.selected_root_paths.is_empty();
    let add_choice = {
        let wizard = wizard.clone();
        let set_wizard = set_wizard.clone();
        let set_scan = set_scan.clone();
        move |choice: String| {
            let selected = match choice.as_str() {
                PICK_FILES_LABEL => pick_file_paths(),
                PICK_FOLDERS_LABEL => pick_folder_paths(),
                _ => Vec::new(),
            };
            append_selected_paths(selected, wizard.clone(), &set_wizard, set_scan.clone());
        }
    };
    let clear_paths = {
        let wizard = wizard.clone();
        let set_wizard = set_wizard.clone();
        let set_scan = set_scan.clone();
        move || {
            let generation = wizard.scan_generation.saturating_add(1);
            let mut next = wizard.clone();
            next.selected_root_paths.clear();
            next.scan_generation = generation;
            set_wizard.call(next);
            set_scan.call(InputScanState::empty(generation));
        }
    };

    let add_button: Element = button("Add paths")
        .accent()
        .icon(SymbolGlyph::Add)
        .menu_flyout(vec![
            menu_item(PICK_FILES_LABEL),
            menu_item(PICK_FOLDERS_LABEL),
        ])
        .on_menu_item_clicked(add_choice)
        .automation_id("reactor-add-paths")
        .into();
    let clear_button: Element = button("Clear paths")
        .icon(SymbolGlyph::Delete)
        .enabled(!wizard.selected_root_paths.is_empty())
        .on_click(clear_paths)
        .automation_id("reactor-clear-paths")
        .into();
    let toolbar: Element = hstack((add_button, clear_button)).spacing(8.0).into();
    let drop_hint: Element = drop_zone_hint(drop_hovering, has_selected_paths);
    let status: Element = input_scan_status(&scan);
    let issue_stack = input_issue_bars(&scan);
    let columns: Element = grid((
        selected_inputs_panel(wizard.clone(), set_wizard.clone(), &scan, set_scan.clone())
            .grid_row(0)
            .grid_column(0),
        transitive_inputs_panel(&scan, drop_hovering)
            .grid_row(0)
            .grid_column(1),
    ))
    .rows([GridLength::Star(1.0)])
    .columns([GridLength::Star(1.0), GridLength::Star(1.45)])
    .column_spacing(16.0)
    .row_spacing(0.0)
    .min_height(320.0)
    .vertical_alignment(VerticalAlignment::Stretch)
    .into();

    let content = grid((
        page_header(
            "Pick input paths",
            "Keep explicit input roots on the left, and let CM gather the transitive files it will process on the right.",
        )
        .grid_row(0),
        toolbar.grid_row(1),
        drop_hint.grid_row(2),
        status.grid_row(3),
        issue_stack.grid_row(4),
        columns.grid_row(5),
    ))
    .rows([
        GridLength::Auto,
        GridLength::Auto,
        GridLength::Auto,
        GridLength::Auto,
        GridLength::Auto,
        GridLength::Star(1.0),
    ])
    .columns([GridLength::Star(1.0)])
    .row_spacing(16.0)
    .max_width(1040.0)
    .automation_id("reactor-pick-input-paths-page");

    page_shell(content)
}

fn drop_zone_hint(drop_hovering: bool, has_selected_paths: bool) -> Element {
    let (title, message, tone, automation_id) = if drop_hovering {
        (
            "Drop to add paths",
            "Release now to add the dragged files and folders as explicit inputs.",
            InfoBarTone::Success,
            "reactor-drop-hover-indicator",
        )
    } else if has_selected_paths {
        (
            "Drag and drop is ready",
            "You can still drag more files or folders into the window at any time.",
            InfoBarTone::Informational,
            "reactor-drop-ready-indicator",
        )
    } else {
        (
            "Drop files or folders here",
            "Drag paths from Explorer into the window, or use Add paths.",
            InfoBarTone::Informational,
            "reactor-drop-empty-indicator",
        )
    };

    info_bar_with_tone(title, message, tone)
        .max_width(960.0)
        .automation_id(automation_id)
        .into()
}

#[derive(Clone, Copy)]
enum InfoBarTone {
    Informational,
    Success,
    Warning,
}

fn info_bar_with_tone(
    title: impl Into<String>,
    message: impl Into<String>,
    tone: InfoBarTone,
) -> InfoBar {
    let info_bar = InfoBar::new(title).message(message).is_closable(false);

    match tone {
        InfoBarTone::Informational => info_bar.informational(),
        InfoBarTone::Success => info_bar.success(),
        InfoBarTone::Warning => info_bar.warning(),
    }
}

fn input_scan_status(scan: &InputScanState) -> Element {
    match scan.status {
        ScanStatus::Empty => info_bar_with_tone(
            "No input paths yet",
            "Add explicit roots on the left or drop files and folders into the window.",
            InfoBarTone::Informational,
        )
        .max_width(960.0)
        .automation_id("reactor-input-status")
        .into(),
        ScanStatus::Loading => hstack((
            ProgressRing::indeterminate()
                .width(18.0)
                .height(18.0)
                .automation_id("reactor-input-loading"),
            text_block(format!(
                "Gathering descendants for {} explicit root{}...",
                scan.roots.len(),
                plural_suffix(scan.roots.len())
            ))
            .font_size(13.0),
        ))
        .spacing(8.0)
        .max_width(960.0)
        .automation_id("reactor-input-status")
        .into(),
        ScanStatus::Ready => info_bar_with_tone(
            "Input discovery ready",
            format!(
                "Resolved {} explicit root{} into {} transitive file{}.",
                scan.roots.len(),
                plural_suffix(scan.roots.len()),
                scan.transitive_entries.len(),
                plural_suffix(scan.transitive_entries.len())
            ),
            InfoBarTone::Success,
        )
        .max_width(960.0)
        .automation_id("reactor-input-status")
        .into(),
        ScanStatus::ReadyWithIssues => info_bar_with_tone(
            "Input discovery finished with issues",
            format!(
                "{} transitive file{} gathered, with {} root issue{}.",
                scan.transitive_entries.len(),
                plural_suffix(scan.transitive_entries.len()),
                scan.issues.len(),
                plural_suffix(scan.issues.len())
            ),
            InfoBarTone::Warning,
        )
        .max_width(960.0)
        .automation_id("reactor-input-status")
        .into(),
    }
}

fn input_issue_bars(scan: &InputScanState) -> Element {
    if scan.issues.is_empty() {
        return Element::Empty;
    }

    vstack(
        scan.issues
            .iter()
            .enumerate()
            .map(|(index, issue)| {
                InfoBar::new(format!("Could not scan {}", issue.root_path.display()))
                    .message(issue.message.clone())
                    .error()
                    .is_closable(false)
                    .automation_id(format!("reactor-input-issue-{index}"))
                    .into()
            })
            .collect::<Vec<Element>>(),
    )
    .spacing(8.0)
    .into()
}

fn selected_inputs_panel(
    wizard: StudioState,
    set_wizard: SetState<StudioState>,
    scan: &InputScanState,
    set_scan: AsyncSetState<InputScanState>,
) -> Element {
    let rows = scan.roots.clone();
    let body: Element = if rows.is_empty() {
        panel_placeholder(
            "No explicit inputs yet",
            "Add paths or drop files and folders into the window to build the input set.",
        )
        .automation_id("reactor-selected-inputs-empty")
        .into()
    } else {
        list_view(rows, move |row, _| {
            let detail = input_root_row_detail(row);
            let remove_path = row.path.clone();
            let remove_row = {
                let wizard = wizard.clone();
                let set_wizard = set_wizard.clone();
                let set_scan = set_scan.clone();
                move || {
                    remove_selected_path(
                        &remove_path,
                        wizard.clone(),
                        &set_wizard,
                        set_scan.clone(),
                    );
                }
            };

            let status_button: Element = button(" ")
                .subtle()
                .icon(root_status_glyph(row.phase))
                .tooltip(detail.clone())
                .on_click(move || {
                    let _ = copy_text_to_clipboard(&detail);
                })
                .width(36.0)
                .height(36.0)
                .automation_name(format!("Copy status for {}", row.path.display()))
                .into();
            let remove_button: Element = button(" ")
                .subtle()
                .icon(SymbolGlyph::Delete)
                .tooltip(format!("Remove {}", row.path.display()))
                .on_click(remove_row)
                .width(36.0)
                .height(36.0)
                .automation_name(format!("Remove {}", row.path.display()))
                .into();

            border(
                grid((
                    status_button.grid_row(0).grid_column(0),
                    vstack((
                        text_block(row.path.display().to_string())
                            .wrap()
                            .font_size(13.0),
                        text_block(root_status_summary(row))
                            .foreground(ThemeRef::SecondaryText)
                            .font_size(12.0)
                            .wrap(),
                    ))
                    .spacing(4.0)
                    .grid_row(0)
                    .grid_column(1),
                    remove_button.grid_row(0).grid_column(2),
                ))
                .rows([GridLength::Auto])
                .columns([GridLength::Auto, GridLength::Star(1.0), GridLength::Auto])
                .column_spacing(8.0),
            )
            .background(ThemeRef::SubtleFill)
            .border_brush(ThemeRef::CardStroke)
            .border_thickness(Thickness::uniform(1.0))
            .corner_radius(8.0)
            .padding(Thickness::uniform(10.0))
            .margin(Thickness::uniform(0.0))
        })
        .with_key_selector(|row| row.path.to_string_lossy().into_owned())
        .selection_mode(SelectionMode::None)
        .build()
        .automation_id("reactor-selected-inputs-list")
    };

    panel_shell(
        "Explicit input paths",
        "Each row is a root that CM will resolve into transitive files.",
        body,
    )
}

fn transitive_inputs_panel(scan: &InputScanState, drop_hovering: bool) -> Element {
    let summary: Element = transitive_status_bar(scan).into();
    let body: Element = if scan.transitive_entries.is_empty() {
        empty_transitive_surface(drop_hovering)
            .automation_id("reactor-transitive-inputs-empty")
            .into()
    } else {
        list_view(scan.transitive_entries.clone(), move |entry, _| {
            border(
                vstack((
                    text_block(entry.path.display().to_string())
                        .wrap()
                        .font_size(13.0),
                    text_block(format!("From {}", entry.source_root.display()))
                        .foreground(ThemeRef::SecondaryText)
                        .font_size(12.0)
                        .wrap(),
                ))
                .spacing(4.0),
            )
            .background(ThemeRef::SubtleFill)
            .border_brush(ThemeRef::CardStroke)
            .border_thickness(Thickness::uniform(1.0))
            .corner_radius(8.0)
            .padding(Thickness::uniform(10.0))
        })
        .with_key_selector(|entry| entry.path.to_string_lossy().into_owned())
        .selection_mode(SelectionMode::None)
        .build()
        .automation_id("reactor-transitive-inputs-list")
    };

    panel_shell(
        "Discovered transitive inputs",
        "Files gathered from the explicit roots. Directory roots contribute descendant files here.",
        vstack((summary, body)).spacing(12.0),
    )
}

fn panel_shell(title: &str, subtitle: &str, body: impl Into<Element>) -> Element {
    border(
        grid((
            vstack((
                text_block(title).font_size(16.0).semibold(),
                text_block(subtitle)
                    .foreground(ThemeRef::SecondaryText)
                    .font_size(12.0)
                    .wrap(),
            ))
            .spacing(6.0)
            .grid_row(0)
            .grid_column(0),
            body.into().grid_row(1).grid_column(0),
        ))
        .rows([GridLength::Auto, GridLength::Star(1.0)])
        .columns([GridLength::Star(1.0)])
        .row_spacing(12.0),
    )
    .background(ThemeRef::SolidBackground)
    .border_brush(ThemeRef::AccentSecondary)
    .border_thickness(Thickness::uniform(1.25))
    .corner_radius(12.0)
    .padding(Thickness::uniform(14.0))
    .min_height(320.0)
    .vertical_alignment(VerticalAlignment::Stretch)
    .into()
}

fn panel_placeholder(title: &str, subtitle: &str) -> Border {
    border(
        vstack((
            text_block(title).font_size(20.0).bold(),
            text_block(subtitle)
                .foreground(ThemeRef::SecondaryText)
                .font_size(13.0)
                .wrap()
                .max_width(360.0),
        ))
        .spacing(10.0)
        .horizontal_alignment(HorizontalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Center),
    )
    .background(ThemeRef::SubtleFill)
    .border_brush(ThemeRef::AccentSecondary)
    .border_thickness(Thickness::uniform(1.0))
    .corner_radius(8.0)
    .padding(Thickness::uniform(18.0))
    .min_height(220.0)
    .vertical_alignment(VerticalAlignment::Stretch)
}

fn empty_transitive_surface(drop_hovering: bool) -> Border {
    let title = if drop_hovering {
        "Release to add files and folders"
    } else {
        "Drop files or folders here"
    };
    let subtitle = if drop_hovering {
        "CM will add the dropped paths as explicit inputs and keep scanning descendants into this list."
    } else {
        "This panel reflects the transitive files CM will process after it resolves the explicit roots."
    };

    border(
        vstack((
            text_block(title).font_size(22.0).bold(),
            text_block(subtitle)
                .foreground(ThemeRef::SecondaryText)
                .font_size(13.0)
                .wrap()
                .max_width(440.0),
        ))
        .spacing(10.0)
        .horizontal_alignment(HorizontalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Center),
    )
    .background(if drop_hovering {
        ThemeRef::SystemSuccessBackground
    } else {
        ThemeRef::SubtleFill
    })
    .border_brush(if drop_hovering {
        ThemeRef::SystemSuccess
    } else {
        ThemeRef::AccentSecondary
    })
    .border_thickness(Thickness::uniform(if drop_hovering { 2.0 } else { 1.0 }))
    .corner_radius(8.0)
    .padding(Thickness::uniform(20.0))
    .min_height(220.0)
    .vertical_alignment(VerticalAlignment::Stretch)
}

fn transitive_status_bar(scan: &InputScanState) -> InfoBar {
    match scan.status {
        ScanStatus::Empty => InfoBar::new("No transitive inputs yet")
            .message("Once you add a root, CM will enumerate descendant files here.")
            .informational()
            .is_closable(false),
        ScanStatus::Loading => {
            let completed = scan
                .roots
                .iter()
                .filter(|root| {
                    matches!(root.phase, RootScanPhase::Succeeded | RootScanPhase::Failed)
                })
                .count();
            InfoBar::new("Gathering descendants")
                .message(format!(
                    "Finished {} of {} roots so far. {} transitive file{} discovered.",
                    completed,
                    scan.roots.len(),
                    scan.transitive_entries.len(),
                    plural_suffix(scan.transitive_entries.len())
                ))
                .informational()
                .is_closable(false)
        }
        ScanStatus::Ready => InfoBar::new("Transitive input list ready")
            .message(format!(
                "{} file{} discovered from {} explicit root{}.",
                scan.transitive_entries.len(),
                plural_suffix(scan.transitive_entries.len()),
                scan.roots.len(),
                plural_suffix(scan.roots.len())
            ))
            .success()
            .is_closable(false),
        ScanStatus::ReadyWithIssues => InfoBar::new("Transitive input list ready with issues")
            .message(format!(
                "{} file{} discovered, with {} scan issue{} to review on the left.",
                scan.transitive_entries.len(),
                plural_suffix(scan.transitive_entries.len()),
                scan.issues.len(),
                plural_suffix(scan.issues.len())
            ))
            .warning()
            .is_closable(false),
    }
}

fn root_status_glyph(phase: RootScanPhase) -> SymbolGlyph {
    match phase {
        RootScanPhase::NotStarted => SymbolGlyph::Help,
        RootScanPhase::InProgress => SymbolGlyph::Sync,
        RootScanPhase::Succeeded => SymbolGlyph::Accept,
        RootScanPhase::Failed => SymbolGlyph::Cancel,
    }
}

fn root_status_summary(row: &InputRootRow) -> String {
    match row.phase {
        RootScanPhase::NotStarted => "Waiting to scan descendants.".to_string(),
        RootScanPhase::InProgress => {
            if let Some(started_at) = row.started_at {
                format!(
                    "Scanning descendants. Running for {}.",
                    format_duration(started_at.elapsed())
                )
            } else {
                "Scanning descendants.".to_string()
            }
        }
        RootScanPhase::Succeeded => format!(
            "{} transitive file{} discovered.",
            row.discovered_count,
            plural_suffix(row.discovered_count)
        ),
        RootScanPhase::Failed => row
            .issue
            .clone()
            .unwrap_or_else(|| "Scanning failed.".to_string()),
    }
}

fn input_root_row_detail(row: &InputRootRow) -> String {
    let state = match row.phase {
        RootScanPhase::NotStarted => "Not started".to_string(),
        RootScanPhase::InProgress => {
            let elapsed = row.started_at.map_or_else(
                || "unknown time".to_string(),
                |started_at| format_duration(started_at.elapsed()),
            );
            format!("In progress for {elapsed}")
        }
        RootScanPhase::Succeeded => format!(
            "Succeeded with {} transitive file{}",
            row.discovered_count,
            plural_suffix(row.discovered_count)
        ),
        RootScanPhase::Failed => format!(
            "Failed: {}",
            row.issue
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string())
        ),
    };

    format!("{}\nStatus: {state}", row.path.display())
}

fn placeholder_step(
    title: &'static str,
    description: &'static str,
    wizard: &StudioState,
) -> Element {
    let selected_roots = wizard.selected_root_paths.len();

    let content = vstack((
        page_header(title, description),
        InfoBar::new("Inputs stay selected")
            .message(format!(
                "{} input root{} currently selected.",
                selected_roots,
                plural_suffix(selected_roots)
            ))
            .informational()
            .is_closable(false)
            .max_width(960.0)
            .max_width(720.0),
    ))
    .spacing(24.0)
    .max_width(920.0);

    page_shell(content)
}

fn install_drop_handler(
    wizard: &StudioState,
    set_wizard: SetState<StudioState>,
    set_scan: AsyncSetState<InputScanState>,
    drop_hovering: bool,
    set_drop_hovering: SetState<bool>,
) {
    if wizard.active_step != WizardStep::PickInputPaths {
        set_window_file_drop_handler(None);
        if drop_hovering {
            set_drop_hovering.call(false);
        }
        return;
    }

    let wizard = wizard.clone();
    let drop_handler = Callback::new(move |paths: Vec<String>| {
        let selected = paths.into_iter().map(PathBuf::from).collect();
        append_selected_paths(selected, wizard.clone(), &set_wizard, set_scan.clone());
    });
    let hover_handler = Callback::new(move |hovering| set_drop_hovering.call(hovering));
    set_window_file_drop_handlers(Some(drop_handler), Some(hover_handler));
}

fn append_selected_paths(
    selected: Vec<PathBuf>,
    wizard: StudioState,
    set_wizard: &SetState<StudioState>,
    set_scan: AsyncSetState<InputScanState>,
) {
    if selected.is_empty() {
        return;
    }

    let mut roots = wizard.selected_root_paths.clone();
    for path in selected {
        if !roots.iter().any(|existing| same_path(existing, &path)) {
            roots.push(path);
        }
    }

    if roots == wizard.selected_root_paths {
        return;
    }

    replace_selected_paths(roots, wizard, set_wizard, set_scan);
}

fn remove_selected_path(
    target: &Path,
    wizard: StudioState,
    set_wizard: &SetState<StudioState>,
    set_scan: AsyncSetState<InputScanState>,
) {
    let roots = wizard
        .selected_root_paths
        .iter()
        .filter(|path| !same_path(path, target))
        .cloned()
        .collect::<Vec<_>>();

    if roots.len() == wizard.selected_root_paths.len() {
        return;
    }

    replace_selected_paths(roots, wizard, set_wizard, set_scan);
}

fn replace_selected_paths(
    roots: Vec<PathBuf>,
    wizard: StudioState,
    set_wizard: &SetState<StudioState>,
    set_scan: AsyncSetState<InputScanState>,
) {
    let generation = wizard.scan_generation.saturating_add(1);
    let mut next = wizard;
    next.selected_root_paths.clone_from(&roots);
    next.scan_generation = generation;
    set_wizard.call(next);

    if roots.is_empty() {
        set_scan.call(InputScanState::empty(generation));
        return;
    }

    set_scan.call(InputScanState::loading(generation, &roots));
    start_input_scan(generation, roots, set_scan);
}

fn start_input_scan(generation: u64, roots: Vec<PathBuf>, set_scan: AsyncSetState<InputScanState>) {
    thread::spawn(move || {
        scan_roots_with_progress(generation, &roots, |state| {
            set_scan.call(state.clone());
        });
    });
}

#[cfg(test)]
fn scan_roots(generation: u64, roots: &[PathBuf]) -> InputScanState {
    if roots.is_empty() {
        return InputScanState::empty(generation);
    }

    scan_roots_with_progress(generation, roots, |_| {})
}

fn scan_roots_with_progress(
    generation: u64,
    roots: &[PathBuf],
    mut on_update: impl FnMut(&InputScanState),
) -> InputScanState {
    let mut state = InputScanState::loading(generation, roots);
    on_update(&state);

    for root_path in roots {
        mark_root_in_progress(&mut state, root_path);
        on_update(&state);

        let result = scan_root(root_path);
        apply_root_scan_result(&mut state, root_path, result);
        on_update(&state);
    }

    state.finalize_status()
}

fn mark_root_in_progress(state: &mut InputScanState, root_path: &Path) {
    if let Some(root) = state
        .roots
        .iter_mut()
        .find(|row| same_path(&row.path, root_path))
    {
        root.phase = RootScanPhase::InProgress;
        root.started_at = Some(Instant::now());
        root.issue = None;
        root.discovered_count = 0;
    }
}

fn apply_root_scan_result(state: &mut InputScanState, root_path: &Path, result: RootScanResult) {
    let discovered_count = result.entries.len();
    if let Some(root) = state
        .roots
        .iter_mut()
        .find(|row| same_path(&row.path, root_path))
    {
        root.phase = if result.issue.is_some() {
            RootScanPhase::Failed
        } else {
            RootScanPhase::Succeeded
        };
        root.discovered_count = discovered_count;
        root.issue.clone_from(&result.issue);
    }

    if let Some(issue) = result.issue {
        state.issues.push(ScanIssue {
            root_path: root_path.to_path_buf(),
            message: issue,
        });
    } else {
        state.transitive_entries.extend(result.entries);
        sort_transitive_entries(&mut state.transitive_entries);
    }

    state.status = if state.roots.iter().any(|root| {
        matches!(
            root.phase,
            RootScanPhase::NotStarted | RootScanPhase::InProgress
        )
    }) {
        ScanStatus::Loading
    } else if state.issues.is_empty() {
        ScanStatus::Ready
    } else {
        ScanStatus::ReadyWithIssues
    };
}

fn scan_root(root_path: &Path) -> RootScanResult {
    let metadata = match fs::symlink_metadata(root_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return RootScanResult {
                entries: Vec::new(),
                issue: Some(format!(
                    "Could not inspect {}: {error}",
                    root_path.display()
                )),
            };
        }
    };

    if metadata.is_file() {
        return RootScanResult {
            entries: vec![TransitiveInputRow {
                path: root_path.to_path_buf(),
                source_root: root_path.to_path_buf(),
            }],
            issue: None,
        };
    }

    if metadata.is_dir() {
        let mut entries = Vec::new();
        if let Err(error) = collect_descendant_files(root_path, root_path, &mut entries) {
            return RootScanResult {
                entries: Vec::new(),
                issue: Some(error),
            };
        }
        sort_transitive_entries(&mut entries);
        return RootScanResult {
            entries,
            issue: None,
        };
    }

    RootScanResult {
        entries: vec![TransitiveInputRow {
            path: root_path.to_path_buf(),
            source_root: root_path.to_path_buf(),
        }],
        issue: None,
    }
}

fn collect_descendant_files(
    current: &Path,
    root_path: &Path,
    entries: &mut Vec<TransitiveInputRow>,
) -> std::result::Result<(), String> {
    let dir_entries = fs::read_dir(current)
        .map_err(|error| format!("Could not read {}: {error}", current.display()))?;

    for entry_result in dir_entries {
        let entry = entry_result.map_err(|error| {
            format!("Could not read an entry in {}: {error}", current.display())
        })?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Could not inspect {}: {error}", path.display()))?;

        if metadata.is_dir() {
            collect_descendant_files(&path, root_path, entries)?;
        } else {
            entries.push(TransitiveInputRow {
                path,
                source_root: root_path.to_path_buf(),
            });
        }
    }

    Ok(())
}

fn sort_transitive_entries(entries: &mut [TransitiveInputRow]) {
    entries.sort_by(|left, right| {
        left.path
            .to_string_lossy()
            .to_lowercase()
            .cmp(&right.path.to_string_lossy().to_lowercase())
            .then_with(|| {
                left.source_root
                    .to_string_lossy()
                    .to_lowercase()
                    .cmp(&right.source_root.to_string_lossy().to_lowercase())
            })
    });
}

fn same_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

fn plural_suffix(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() >= 60 {
        format!("{}m {}s", duration.as_secs() / 60, duration.as_secs() % 60)
    } else if duration.as_secs() >= 1 {
        format!("{}s", duration.as_secs())
    } else {
        format!("{}ms", duration.as_millis())
    }
}

#[cfg(windows)]
fn copy_text_to_clipboard(text: &str) -> std::result::Result<(), String> {
    let encoded = OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    let byte_len = encoded.len() * std::mem::size_of::<u16>();

    unsafe {
        OpenClipboard(None).map_err(|error| format!("Could not open clipboard: {error}"))?;
    }
    let _clipboard_guard = ClipboardGuard;

    unsafe {
        EmptyClipboard().map_err(|error| format!("Could not clear clipboard: {error}"))?;
    }

    let memory = unsafe {
        GlobalAlloc(GMEM_MOVEABLE, byte_len)
            .map_err(|error| format!("Could not allocate clipboard memory: {error}"))?
    };
    if memory.is_invalid() {
        return Err("Could not allocate clipboard memory.".to_string());
    }

    let locked = unsafe { GlobalLock(memory) };
    if locked.is_null() {
        return Err("Could not lock clipboard memory.".to_string());
    }

    unsafe {
        std::ptr::copy_nonoverlapping(encoded.as_ptr().cast::<u8>(), locked.cast::<u8>(), byte_len);
    }

    unsafe {
        let _ = GlobalUnlock(memory);
    }

    unsafe {
        SetClipboardData(CF_UNICODETEXT_FORMAT, Some(HANDLE(memory.0)))
            .map_err(|error| format!("Could not set clipboard contents: {error}"))?;
    }

    Ok(())
}

#[cfg(not(windows))]
fn copy_text_to_clipboard(_text: &str) -> std::result::Result<(), String> {
    Err("Clipboard copy is only available on Windows.".to_string())
}

fn theme_toggle_button(is_dark: bool) -> Button {
    let glyph = if is_dark { "\u{E706}" } else { "\u{E708}" };
    let automation_name = if is_dark {
        "Switch to light theme"
    } else {
        "Switch to dark theme"
    };
    button(glyph)
        .on_click(move || {
            set_requested_theme(if is_dark {
                RequestedTheme::Light
            } else {
                RequestedTheme::Dark
            });
        })
        .font_family("Segoe MDL2 Assets")
        .font_size(14.0)
        .width(40.0)
        .height(36.0)
        .padding(0.0)
        .automation_id("theme-toggle")
        .automation_name(automation_name)
}

fn page_padding() -> Thickness {
    Thickness {
        left: 36.0,
        top: 40.0,
        right: 36.0,
        bottom: 36.0,
    }
}

fn page_shell(content: impl Into<Element>) -> Element {
    border(content.into())
        .background(ThemeRef::SolidBackground)
        .padding(page_padding())
        .into()
}

fn studio_drop_overlay(drop_hovering: bool) -> Element {
    if !drop_hovering {
        return Element::Empty;
    }

    border(
        border(
            vstack((
                text_block("Drop files or folders anywhere in this window")
                    .font_size(26.0)
                    .bold()
                    .horizontal_alignment(HorizontalAlignment::Center),
                text_block(
                    "Release now to add the dragged paths. CM will keep scanning folder descendants into the transitive input list.",
                )
                .foreground(ThemeRef::SecondaryText)
                .font_size(14.0)
                .horizontal_alignment(HorizontalAlignment::Center)
                .max_width(560.0)
                .wrap(),
            ))
            .spacing(12.0),
        )
        .background(ThemeRef::SolidBackground)
        .border_brush(ThemeRef::SystemSuccess)
        .border_thickness(Thickness::uniform(2.0))
        .corner_radius(16.0)
        .padding(Thickness::uniform(28.0))
        .max_width(640.0)
        .horizontal_alignment(HorizontalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Center),
    )
    .background(ThemeRef::SubtleFill)
    .opacity(0.96)
    .grid_row(0)
    .grid_row_span(2)
    .automation_id("reactor-studio-drop-overlay")
    .into()
}

fn page_header(title: &'static str, description: &'static str) -> Element {
    vstack((
        text_block(title).font_size(28.0).bold(),
        text_block(description)
            .foreground(ThemeRef::SecondaryText)
            .horizontal_alignment(HorizontalAlignment::Left)
            .max_width(820.0)
            .wrap(),
    ))
    .spacing(8.0)
    .into()
}

fn launch_button(
    title: &'static str,
    subtitle: &'static str,
    icon: SymbolGlyph,
    on_click: impl Fn() + 'static,
) -> Button {
    button(format!("{title}\n{subtitle}"))
        .icon(icon)
        .on_click(on_click)
        .min_height(78.0)
        .max_width(320.0)
        .horizontal_alignment(HorizontalAlignment::Stretch)
}

fn launch_gui_mode_status(mode: &str) -> String {
    match crate::windows_cli::shell::launch_gui_mode(mode) {
        Ok(pid) => format!("Launched cm gui --mode {mode} as process {pid}."),
        Err(error) => format!("Failed to launch cm gui --mode {mode}: {error}"),
    }
}

fn run_cm_search(query: &str, sku: &str) -> String {
    match crate::product_search::search_pretty(query, sku) {
        Ok(output) => output,
        Err(error) => error,
    }
}

#[cfg(windows)]
fn pick_file_paths() -> Vec<PathBuf> {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() };
    let result = unsafe { pick_file_paths_inner() }.unwrap_or_default();
    if initialized {
        unsafe { CoUninitialize() };
    }
    result
}

#[cfg(windows)]
unsafe fn pick_file_paths_inner() -> Option<Vec<PathBuf>> {
    let dialog: IFileOpenDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()? };

    let options = unsafe { dialog.GetOptions().ok()? };
    unsafe {
        dialog
            .SetOptions(options | FOS_ALLOWMULTISELECT | FOS_FORCEFILESYSTEM)
            .ok()?;
    }
    unsafe { dialog.Show(None).ok()? };

    let results = unsafe { dialog.GetResults().ok()? };
    let count = unsafe { results.GetCount().ok()? };
    let mut paths = Vec::with_capacity(count as usize);
    for index in 0..count {
        let item = unsafe { results.GetItemAt(index).ok()? };
        let path = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH).ok()? };
        if let Ok(path) = unsafe { path.to_string() } {
            paths.push(PathBuf::from(path));
        }
    }
    Some(paths)
}

#[cfg(not(windows))]
fn pick_file_paths() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(windows)]
fn pick_folder_paths() -> Vec<PathBuf> {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() };
    let result = unsafe { pick_folder_paths_inner() }.unwrap_or_default();
    if initialized {
        unsafe { CoUninitialize() };
    }
    result
}

#[cfg(windows)]
unsafe fn pick_folder_paths_inner() -> Option<Vec<PathBuf>> {
    let dialog: IFileOpenDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()? };

    let options = unsafe { dialog.GetOptions().ok()? };
    unsafe {
        dialog
            .SetOptions(options | FOS_PICKFOLDERS | FOS_ALLOWMULTISELECT | FOS_FORCEFILESYSTEM)
            .ok()?;
    }
    unsafe { dialog.Show(None).ok()? };

    let results = unsafe { dialog.GetResults().ok()? };
    let count = unsafe { results.GetCount().ok()? };
    let mut paths = Vec::with_capacity(count as usize);
    for index in 0..count {
        let item = unsafe { results.GetItemAt(index).ok()? };
        let path = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH).ok()? };
        if let Ok(path) = unsafe { path.to_string() } {
            paths.push(PathBuf::from(path));
        }
    }
    Some(paths)
}

#[cfg(not(windows))]
fn pick_folder_paths() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    #[test]
    fn scan_roots_collects_descendant_files_for_directories() {
        let root = temp_test_dir("reactor_scan_flattened");
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("top.txt"), "top").unwrap();
        fs::write(root.join("nested").join("child.txt"), "child").unwrap();

        let scan = scan_roots(42, std::slice::from_ref(&root));

        assert_eq!(scan.generation, 42);
        assert_eq!(scan.status, ScanStatus::Ready);
        assert_eq!(scan.roots.len(), 1);
        assert_eq!(scan.roots[0].phase, RootScanPhase::Succeeded);
        assert_eq!(scan.roots[0].discovered_count, 2);
        assert_eq!(scan.transitive_entries.len(), 2);
        assert!(
            scan.transitive_entries
                .iter()
                .any(|entry| entry.path.ends_with("top.txt"))
        );
        assert!(
            scan.transitive_entries
                .iter()
                .any(|entry| entry.path.ends_with("child.txt"))
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn scan_roots_marks_missing_root_as_failed() {
        let root = temp_test_dir("reactor_missing_root");
        let scan = scan_roots(7, std::slice::from_ref(&root));

        assert_eq!(scan.status, ScanStatus::ReadyWithIssues);
        assert_eq!(scan.roots.len(), 1);
        assert_eq!(scan.roots[0].phase, RootScanPhase::Failed);
        assert_eq!(scan.issues.len(), 1);
        assert!(scan.transitive_entries.is_empty());
    }

    #[test]
    fn scan_roots_keeps_file_roots_as_transitive_entries() {
        let root = temp_test_dir("reactor_file_root");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("single.webp");
        fs::write(&file, "file").unwrap();

        let scan = scan_roots(9, std::slice::from_ref(&file));

        assert_eq!(scan.status, ScanStatus::Ready);
        assert_eq!(scan.roots.len(), 1);
        assert_eq!(scan.roots[0].phase, RootScanPhase::Succeeded);
        assert_eq!(scan.transitive_entries.len(), 1);
        assert_eq!(scan.transitive_entries[0].path, file);

        fs::remove_dir_all(root).unwrap();
    }

    fn temp_test_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), nanos))
    }
}
