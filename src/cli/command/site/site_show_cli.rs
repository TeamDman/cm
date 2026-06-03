use arbitrary::Arbitrary;
use facet::Facet;

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct SiteShowArgs {}

impl SiteShowArgs {
    /// # Errors
    ///
    /// This function does not return any errors.
    pub fn invoke(self) -> eyre::Result<()> {
        // Use the static SITE_ID for the current value
        println!("Site: {}", crate::SITE_ID.as_str());
        Ok(())
    }
}
