use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};

/// Set the active site by id
#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct SiteSetArgs {
    /// Site identifier to set
    #[facet(args::positional)]
    pub id: String,
}

impl SiteSetArgs {
    /// # Errors
    ///
    /// Returns an error if setting the site fails.
    pub fn invoke(self) -> eyre::Result<()> {
        // Persist the selection to disk so next runs pick it up
        crate::SiteId::set_to(&self.id)?;
        println!("Setting site to: {}", self.id);
        Ok(())
    }
}
