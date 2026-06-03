use crate::app_home::APP_HOME;
use crate::rename_rules::list_rules;
use crate::rename_rules::remove_rule;
use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};
use uuid::Uuid;

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct RenameRuleRemoveArgs {
    /// Remove all rules
    #[facet(args::named, default)]
    pub all: bool,
    /// Rule id (UUID). If omitted and --all is specified, removes all rules.
    #[facet(default, args::positional)]
    pub id: Option<String>,
}

impl RenameRuleRemoveArgs {
    /// # Errors
    ///
    /// Returns an error if removing the rename rules fails.
    pub fn invoke(self) -> eyre::Result<()> {
        let listed = list_rules(&APP_HOME)?;
        if self.all {
            if self.id.is_some() {
                println!("Cannot specify an id with --all");
                return Ok(());
            }
            let mut removed = 0usize;
            for (_index, rule) in listed {
                if remove_rule(&APP_HOME, rule.id)? {
                    removed += 1;
                }
            }
            println!("Removed {removed} rules");
        } else if let Some(id_str) = self.id {
            match Uuid::parse_str(&id_str) {
                Ok(id) => {
                    if remove_rule(&APP_HOME, id)? {
                        println!("Removed rule {id}");
                    } else {
                        println!("No rule {id}");
                    }
                }
                Err(_) => {
                    println!("Invalid UUID: {id_str}");
                }
            }
        } else {
            println!("Specify an id or use --all to remove all rules");
        }
        Ok(())
    }
}
