#[cfg(windows)]
mod app;

pub mod plan;

#[cfg(windows)]
use app::InitialSurface;

/// Launch the Reactor main menu surface.
///
/// # Errors
///
/// Returns an error if the GUI runtime cannot be created or the surface fails to run.
#[cfg(windows)]
pub fn run_main_menu() -> eyre::Result<()> {
    app::run(InitialSurface::MainMenu)
}

/// Launch the Reactor studio surface.
///
/// # Errors
///
/// Returns an error if the GUI runtime cannot be created or the surface fails to run.
#[cfg(windows)]
pub fn run_studio() -> eyre::Result<()> {
    app::run(InitialSurface::Studio)
}

/// Launch the Reactor product-search surface.
///
/// # Errors
///
/// Returns an error if the GUI runtime cannot be created or the surface fails to run.
#[cfg(windows)]
pub fn run_product_search() -> eyre::Result<()> {
    app::run(InitialSurface::ProductSearch)
}

/// Launch the Reactor main menu surface.
///
/// # Errors
///
/// Always returns an error on non-Windows platforms.
#[cfg(not(windows))]
pub fn run_main_menu() -> eyre::Result<()> {
    eyre::bail!("Reactor GUI is only supported on Windows")
}

/// Launch the Reactor studio surface.
///
/// # Errors
///
/// Always returns an error on non-Windows platforms.
#[cfg(not(windows))]
pub fn run_studio() -> eyre::Result<()> {
    eyre::bail!("Reactor GUI is only supported on Windows")
}

/// Launch the Reactor product-search surface.
///
/// # Errors
///
/// Always returns an error on non-Windows platforms.
#[cfg(not(windows))]
pub fn run_product_search() -> eyre::Result<()> {
    eyre::bail!("Reactor GUI is only supported on Windows")
}
