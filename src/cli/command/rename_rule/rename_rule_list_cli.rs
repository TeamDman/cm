use crate::app_home::APP_HOME;
use crate::rename_rules::list_rules;
use arbitrary::Arbitrary;
use facet::Facet;
use tracing::info;

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct RenameRuleListArgs {}

impl RenameRuleListArgs {
    /// # Errors
    ///
    /// Returns an error if listing the rename rules fails.
    pub fn invoke(self) -> eyre::Result<()> {
        let listed = list_rules(&APP_HOME)?;
        info!("Found {} rename rules", listed.len());
        for (_i, rule) in listed {
            println!("{}: {}", rule.id, rule);
        }
        Ok(())
    }
}
