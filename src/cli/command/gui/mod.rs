use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use clap::ValueEnum;
use std::ffi::OsString;

#[derive(ValueEnum, Arbitrary, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GuiMode {
    #[default]
    MainMenu,
    StudioV1,
    StudioV2,
    ProductSearch,
}

impl GuiMode {
    fn as_arg(self) -> &'static str {
        match self {
            Self::MainMenu => "main-menu",
            Self::StudioV1 => "studio-v1",
            Self::StudioV2 => "studio-v2",
            Self::ProductSearch => "product-search",
        }
    }
}

#[derive(Args, Arbitrary, Clone, PartialEq, Debug, Default)]
pub struct GuiArgs {
    /// GUI surface to open
    #[clap(long, value_enum, default_value_t = GuiMode::MainMenu)]
    pub mode: GuiMode,
}

impl GuiArgs {
    /// # Errors
    ///
    /// Returns an error if the GUI runtime cannot be created or the GUI fails to run.
    pub fn invoke(self) -> eyre::Result<()> {
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

impl ToArgs for GuiArgs {
    fn to_args(&self) -> Vec<OsString> {
        if self.mode == GuiMode::MainMenu {
            Vec::new()
        } else {
            vec!["--mode".into(), self.mode.as_arg().into()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_mode_to_args_skips_default_main_menu() {
        assert_eq!(GuiArgs::default().to_args(), Vec::<OsString>::new());
    }

    #[test]
    fn gui_mode_to_args_includes_direct_surface() {
        assert_eq!(
            GuiArgs {
                mode: GuiMode::ProductSearch,
            }
            .to_args(),
            vec![OsString::from("--mode"), OsString::from("product-search")]
        );
    }
}
