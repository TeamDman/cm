use crate::app_home::APP_HOME;
use crate::rename_rules::RenameRule;
use crate::rename_rules::add_rule;
use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};
use uuid::Uuid;

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct RenameRuleAddArgs {
    /// Find pattern (regex)
    #[facet(args::positional)]
    pub find: String,
    /// Replacement string (optional)
    #[facet(default = "", args::positional)]
    pub replace: String,
    /// Only apply when name is too long (longer than max name length)
    #[facet(args::named, default)]
    pub only_when_too_long: bool,
    /// Case-sensitive match (default is case-insensitive)
    #[facet(args::named, default)]
    pub case_sensitive: bool,
    /// Create the rule in a disabled state
    #[facet(args::named, default)]
    pub disabled: bool,
}

impl RenameRuleAddArgs {
    /// # Errors
    ///
    /// Returns an error if adding the rename rule fails.
    pub fn invoke(self) -> eyre::Result<()> {
        let rule = RenameRule {
            id: Uuid::new_v4(),
            find: self.find,
            replace: self.replace,
            enabled: !self.disabled,
            case_sensitive: self.case_sensitive,
            only_when_name_too_long: self.only_when_too_long,
        };
        let id = add_rule(&APP_HOME, &rule)?;
        println!("Added rule {id}: {rule}");
        Ok(())
    }
}
