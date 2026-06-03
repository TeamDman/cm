use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};

#[derive(Facet, Arbitrary, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[facet(rename_all = "kebab-case")]
#[repr(u8)]
pub enum GuiMode {
    #[default]
    MainMenu,
    StudioV1,
    StudioV2,
    ProductSearch,
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
            GuiMode::MainMenu => crate::egui::run_gui(),
            GuiMode::StudioV1 => {
                crate::egui::run_gui_with_initial_tool(Some(crate::egui::ToolChoice::V1))
            }
            GuiMode::StudioV2 => crate::reactor::run_studio_v2(),
            GuiMode::ProductSearch => {
                crate::egui::run_gui_with_initial_tool(Some(crate::egui::ToolChoice::ProductSearch))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use figue::ToArgs;
    use std::ffi::OsString;

    #[test]
    fn gui_mode_to_args_skips_default_main_menu() {
        assert_eq!(
            GuiArgs::default()
                .to_args()
                .expect("gui args should serialize"),
            vec![OsString::from("--mode"), OsString::from("main-menu")]
        );
    }

    #[test]
    fn gui_mode_to_args_includes_direct_surface() {
        assert_eq!(
            GuiArgs {
                mode: GuiMode::ProductSearch,
            }
            .to_args()
            .expect("gui args should serialize"),
            vec![OsString::from("--mode"), OsString::from("product-search")]
        );
    }
}
