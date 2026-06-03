use crate::app_home::APP_HOME;
use crate::inputs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct InputRemoveArgs {
    /// Glob pattern for paths to remove
    #[facet(args::positional)]
    pub pattern: String,
}

impl InputRemoveArgs {
    /// # Errors
    ///
    /// Returns an error if removing the input paths fails.
    pub fn invoke(self) -> eyre::Result<()> {
        let removed = inputs::remove_from_glob(&APP_HOME, &self.pattern)?;
        for p in &removed {
            println!("Removed: {}", p.display());
        }
        if removed.is_empty() {
            println!("No persisted inputs matched '{}'.", self.pattern);
        }
        Ok(())
    }
}
