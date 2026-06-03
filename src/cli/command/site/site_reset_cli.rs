use crate::SiteId;
use arbitrary::Arbitrary;
use facet::Facet;

/// Reset the site to the default value and persist it to the config file
#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct SiteResetArgs {}

impl SiteResetArgs {
    /// # Errors
    ///
    /// Returns an error if resetting the site fails.
    pub fn invoke(self) -> eyre::Result<()> {
        SiteId::set_to(SiteId::DEFAULT)?;
        println!("Reset site to default: {}", SiteId::DEFAULT);
        Ok(())
    }
}
