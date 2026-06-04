use eyre::eyre;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
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

#[derive(Clone, Debug, PartialEq)]
struct InputScanState {
    generation: u64,
    status: ScanStatus,
    nodes: Vec<InputTreeNode>,
    errors: Vec<String>,
}

impl InputScanState {
    fn empty(generation: u64) -> Self {
        Self {
            generation,
            status: ScanStatus::Empty,
            nodes: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn loading(generation: u64) -> Self {
        Self {
            generation,
            status: ScanStatus::Loading,
            nodes: Vec::new(),
            errors: Vec::new(),
        }
    }
}

impl Default for InputScanState {
    fn default() -> Self {
        Self::empty(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum InputTreeNodeKind {
    Directory,
    File,
    Status,
}

#[derive(Clone, Debug, PartialEq)]
struct InputTreeNode {
    label: String,
    kind: InputTreeNodeKind,
    path: Option<PathBuf>,
    children: Vec<InputTreeNode>,
    expanded: bool,
}

impl InputTreeNode {
    fn directory(label: String, path: PathBuf, children: Vec<Self>, expanded: bool) -> Self {
        Self {
            label,
            kind: InputTreeNodeKind::Directory,
            path: Some(path),
            children,
            expanded,
        }
    }

    fn file(label: String, path: PathBuf) -> Self {
        Self {
            label,
            kind: InputTreeNodeKind::File,
            path: Some(path),
            children: Vec::new(),
            expanded: false,
        }
    }

    fn status(label: String) -> Self {
        Self {
            label,
            kind: InputTreeNodeKind::Status,
            path: None,
            children: Vec::new(),
            expanded: false,
        }
    }
}

#[derive(Clone, PartialEq)]
struct AppModeProps {
    set_mode: SetState<AppMode>,
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

fn studio(props: &AppModeProps, cx: &mut RenderCx) -> Element {
    let is_dark = matches!(cx.use_color_scheme(), ColorScheme::Dark);
    let (wizard, set_wizard) = cx.use_state(StudioState::default());
    let (scan, set_scan) = cx.use_async_state(InputScanState::default());
    let (drop_hovering, set_drop_hovering) = cx.use_state(false);
    let (is_pane_open, set_pane_open) = cx.use_state(true);
    let scan = if scan.generation == wizard.scan_generation {
        scan
    } else {
        InputScanState::loading(wizard.scan_generation)
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

    grid((title_bar.grid_row(0), navigation.grid_row(1)))
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
    clippy::too_many_lines,
    reason = "wizard input step is a single UI builder slice"
)]
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
            append_selected_paths(
                selected,
                wizard.clone(),
                set_wizard.clone(),
                set_scan.clone(),
            );
        }
    };
    let clear_paths = {
        let wizard = wizard.clone();
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
    let drop_hint: Element = if drop_hovering {
        InfoBar::new("Ready to add paths")
            .message("Release to add the dropped files and folders.")
            .success()
            .is_closable(false)
            .max_width(960.0)
            .automation_id("reactor-drop-hover-indicator")
            .into()
    } else {
        Element::Empty
    };
    let status: Element = input_scan_status(&wizard, &scan);
    let roots_summary: Element = selected_roots_summary(&wizard.selected_root_paths);
    let tree_nodes = scan.nodes.iter().map(to_tree_node_def).collect::<Vec<_>>();
    let tree: Element = tree_view(tree_nodes)
        .selection_mode(TreeSelectionMode::Single)
        .on_item_invoked(|_| {})
        .automation_id("reactor-input-tree")
        .into();
    let tree_surface: Element = border(scroll_view(tree).height(320.0))
        .border_brush(if drop_hovering {
            ThemeRef::Accent
        } else {
            ThemeRef::CardStroke
        })
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(6.0)
        .padding(Thickness::uniform(8.0))
        .max_width(960.0)
        .into();

    let content = grid((
        page_header(
            "Pick input paths",
            "Select image files and folders. Folder descendants are scanned into the tree without changing egui inputs.",
        )
        .grid_row(0),
        toolbar.grid_row(1),
        drop_hint.grid_row(2),
        status.grid_row(3),
        roots_summary.grid_row(4),
        tree_surface.grid_row(5),
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
    .max_width(1040.0)
    .automation_id("reactor-pick-input-paths-page");

    page_shell(content)
}

fn selected_roots_summary(roots: &[PathBuf]) -> Element {
    if roots.is_empty() {
        return text_block("No selected roots yet.")
            .foreground(ThemeRef::SecondaryText)
            .font_size(13.0)
            .wrap()
            .max_width(960.0)
            .into();
    }

    let rows = roots
        .iter()
        .map(|path| {
            text_block(path.display().to_string())
                .font_size(12.0)
                .wrap()
                .into()
        })
        .collect::<Vec<Element>>();

    vstack((
        text_block(format!("Selected roots: {}", roots.len()))
            .font_size(13.0)
            .semibold(),
        vstack(rows).spacing(4.0),
    ))
    .spacing(6.0)
    .max_width(960.0)
    .into()
}

fn input_scan_status(wizard: &StudioState, scan: &InputScanState) -> Element {
    match scan.status {
        ScanStatus::Empty => InfoBar::new("No input paths")
            .message("Use Add paths to choose files or folders.")
            .informational()
            .is_closable(false)
            .max_width(960.0)
            .automation_id("reactor-input-status")
            .into(),
        ScanStatus::Loading => hstack((
            ProgressRing::indeterminate()
                .width(18.0)
                .height(18.0)
                .automation_id("reactor-input-loading"),
            text_block(format!(
                "Scanning {} selected root{}...",
                wizard.selected_root_paths.len(),
                plural_suffix(wizard.selected_root_paths.len())
            ))
            .font_size(13.0),
        ))
        .spacing(8.0)
        .max_width(960.0)
        .automation_id("reactor-input-status")
        .into(),
        ScanStatus::Ready => InfoBar::new("Input tree ready")
            .message(format!(
                "Showing {} selected root{} and their descendants.",
                wizard.selected_root_paths.len(),
                plural_suffix(wizard.selected_root_paths.len())
            ))
            .success()
            .is_closable(false)
            .max_width(960.0)
            .automation_id("reactor-input-status")
            .into(),
        ScanStatus::ReadyWithIssues => InfoBar::new("Input tree ready with issues")
            .message(error_summary(&scan.errors))
            .warning()
            .is_closable(false)
            .max_width(960.0)
            .automation_id("reactor-input-status")
            .into(),
    }
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
        append_selected_paths(
            selected,
            wizard.clone(),
            set_wizard.clone(),
            set_scan.clone(),
        );
    });
    let hover_handler = Callback::new(move |hovering| set_drop_hovering.call(hovering));
    set_window_file_drop_handlers(Some(drop_handler), Some(hover_handler));
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "state handles are moved into async follow-up work"
)]
fn append_selected_paths(
    selected: Vec<PathBuf>,
    wizard: StudioState,
    set_wizard: SetState<StudioState>,
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

    let generation = wizard.scan_generation.saturating_add(1);
    let mut next = wizard;
    next.selected_root_paths.clone_from(&roots);
    next.scan_generation = generation;
    set_wizard.call(next);
    set_scan.call(InputScanState::loading(generation));
    start_input_scan(generation, roots, set_scan);
}

fn start_input_scan(generation: u64, roots: Vec<PathBuf>, set_scan: AsyncSetState<InputScanState>) {
    thread::spawn(move || {
        set_scan.call(scan_roots(generation, &roots));
    });
}

fn scan_roots(generation: u64, roots: &[PathBuf]) -> InputScanState {
    if roots.is_empty() {
        return InputScanState::empty(generation);
    }

    let mut errors = Vec::new();
    let nodes = roots
        .iter()
        .map(|path| scan_path(path, true, &mut errors))
        .collect::<Vec<_>>();
    let status = if errors.is_empty() {
        ScanStatus::Ready
    } else {
        ScanStatus::ReadyWithIssues
    };

    InputScanState {
        generation,
        status,
        nodes,
        errors,
    }
}

fn scan_path(path: &Path, is_root: bool, errors: &mut Vec<String>) -> InputTreeNode {
    let label = if is_root {
        path.display().to_string()
    } else {
        path_label(path)
    };

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            errors.push(format!("{}: {error}", path.display()));
            return InputTreeNode::directory(
                label,
                path.to_path_buf(),
                vec![InputTreeNode::status(format!("Could not inspect: {error}"))],
                true,
            );
        }
    };

    if metadata.is_file() {
        return InputTreeNode::file(label, path.to_path_buf());
    }

    if !metadata.is_dir() {
        return InputTreeNode::file(label, path.to_path_buf());
    }

    let children = match fs::read_dir(path) {
        Ok(entries) => {
            let mut children = entries
                .map(|entry| match entry {
                    Ok(entry) => scan_path(&entry.path(), false, errors),
                    Err(error) => {
                        errors.push(format!("{}: {error}", path.display()));
                        InputTreeNode::status(format!("Could not read entry: {error}"))
                    }
                })
                .collect::<Vec<_>>();
            sort_tree_nodes(&mut children);
            children
        }
        Err(error) => {
            errors.push(format!("{}: {error}", path.display()));
            vec![InputTreeNode::status(format!(
                "Could not read folder: {error}"
            ))]
        }
    };

    InputTreeNode::directory(label, path.to_path_buf(), children, true)
}

fn sort_tree_nodes(nodes: &mut [InputTreeNode]) {
    nodes.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
    });
}

fn to_tree_node_def(node: &InputTreeNode) -> TreeNodeDef {
    let mut def = tree_node(node.label.clone());
    if node.expanded {
        def = def.expanded();
    }
    if !node.children.is_empty() {
        def = def.children(node.children.iter().map(to_tree_node_def).collect());
    }
    def
}

fn path_label(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
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

fn error_summary(errors: &[String]) -> String {
    match errors {
        [] => "No scan errors.".to_string(),
        [one] => one.clone(),
        [first, rest @ ..] => format!(
            "{first} and {} more issue{}.",
            rest.len(),
            plural_suffix(rest.len())
        ),
    }
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
    fn scan_roots_builds_directory_tree_with_descendants() {
        let root = temp_test_dir("reactor_scan_tree");
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("top.txt"), "top").unwrap();
        fs::write(root.join("nested").join("child.txt"), "child").unwrap();

        let scan = scan_roots(42, std::slice::from_ref(&root));

        assert_eq!(scan.generation, 42);
        assert_eq!(scan.status, ScanStatus::Ready);
        assert_eq!(scan.nodes.len(), 1);
        let root_node = &scan.nodes[0];
        assert_eq!(root_node.kind, InputTreeNodeKind::Directory);
        assert!(root_node.children.iter().any(|node| node.label == "nested"));
        assert!(
            root_node
                .children
                .iter()
                .any(|node| node.label == "top.txt")
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn scan_roots_keeps_missing_root_as_status_child() {
        let root = temp_test_dir("reactor_missing_root");
        let scan = scan_roots(7, std::slice::from_ref(&root));

        assert_eq!(scan.status, ScanStatus::ReadyWithIssues);
        assert_eq!(scan.nodes.len(), 1);
        assert_eq!(scan.nodes[0].kind, InputTreeNodeKind::Directory);
        assert_eq!(scan.nodes[0].children[0].kind, InputTreeNodeKind::Status);
    }

    fn temp_test_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), nanos))
    }
}
