use crate::app_home::APP_HOME;
use crate::inputs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct InputAddArgs {
    /// Glob pattern to add (file paths matched will be canonicalized and stored)
    #[facet(args::positional)]
    pub pattern: String,
}

impl InputAddArgs {
    /// # Errors
    ///
    /// Returns an error if adding the input paths fails.
    pub fn invoke(self) -> eyre::Result<()> {
        let added = inputs::add_from_glob(&APP_HOME, &self.pattern)?;
        for p in &added {
            println!("Added: {}", p.display());
        }
        if added.is_empty() {
            println!("No matching paths were found for '{}'.", self.pattern);
        }
        Ok(())
    }
}
