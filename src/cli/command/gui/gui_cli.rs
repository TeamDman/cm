use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};

#[derive(Facet, Arbitrary, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[facet(rename_all = "kebab-case")]
#[repr(u8)]
pub enum GuiMode {
    #[default]
    ReactorMainMenu,
    EguiMainMenu,
    EguiStudio,
    ReactorStudio,
    EguiProductSearch,
    ReactorProductSearch,
}

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug, Default)]
#[facet(rename_all = "kebab-case")]
pub struct GuiArgs {
    /// GUI surface to open
    #[facet(args::named, default)]
    pub mode: GuiMode,
}

impl GuiArgs {
    /// # Errors
    ///
    /// Returns an error if the GUI runtime cannot be created or the GUI fails to run.
    pub fn invoke(self) -> eyre::Result<()> {
        #[cfg(windows)]
        crate::windows_cli::console::hide_default_console_or_attach_ctrl_handler()?;

        match self.mode {
            GuiMode::ReactorMainMenu => crate::reactor::run_main_menu(),
            GuiMode::EguiMainMenu => crate::egui::run_gui(),
            GuiMode::EguiStudio => {
                crate::egui::run_gui_with_initial_tool(Some(crate::egui::ToolChoice::Studio))
            }
            GuiMode::ReactorStudio => crate::reactor::run_studio(),
            GuiMode::EguiProductSearch => {
                crate::egui::run_gui_with_initial_tool(Some(crate::egui::ToolChoice::ProductSearch))
            }
            GuiMode::ReactorProductSearch => crate::reactor::run_product_search(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use figue::ToArgs;
    use std::ffi::OsString;

    #[test]
    fn gui_default_mode_serializes_to_reactor_main_menu() {
        assert_eq!(
            GuiArgs::default()
                .to_args()
                .expect("gui args should serialize"),
            vec![
                OsString::from("--mode"),
                OsString::from("reactor-main-menu")
            ]
        );
    }

    #[test]
    fn gui_mode_to_args_includes_direct_surface() {
        assert_eq!(
            GuiArgs {
                mode: GuiMode::EguiProductSearch,
            }
            .to_args()
            .expect("gui args should serialize"),
            vec![
                OsString::from("--mode"),
                OsString::from("egui-product-search")
            ]
        );
    }
}
