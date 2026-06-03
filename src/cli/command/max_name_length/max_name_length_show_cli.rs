use crate::MaxNameLength;
use crate::app_home::AppHome;
use arbitrary::Arbitrary;
use facet::Facet;

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct MaxNameLengthShowArgs {}

impl MaxNameLengthShowArgs {
    /// # Errors
    ///
    /// Returns an error if loading the max name length fails.
    pub fn invoke(self, app_home: &AppHome) -> eyre::Result<()> {
        println!(
            "Max name length: {}",
            MaxNameLength::load(app_home)?.as_usize()
        );
        Ok(())
    }
}
