use crate::app_home::APP_HOME;
use arbitrary::Arbitrary;
use facet::Facet;

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct RenameRulePathArgs {}

impl RenameRulePathArgs {
    /// # Errors
    ///
    /// Returns an error if getting the rules directory fails.
    pub fn invoke(self) -> eyre::Result<()> {
        let path = crate::rename_rules::rules_dir(&APP_HOME)?;
        println!("{}", path.display());
        Ok(())
    }
}
