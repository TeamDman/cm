use crate::app_home::APP_HOME;
use crate::cli::to_args::ToArgs;
use crate::rename_rules::RenameRule;
use crate::rename_rules::add_rule;
use crate::rename_rules::list_rules;
use crate::rename_rules::remove_rule;
use arbitrary::Arbitrary;
use clap::Args;
use clap::Subcommand;
use std::ffi::OsString;
use tracing::info;
use uuid::Uuid;

#[derive(Subcommand, Clone, Arbitrary, PartialEq, Debug)]
pub enum RenameRuleCommand {
    /// Add a rename rule
    Add(RenameRuleAddArgs),

    /// List rules
    List(RenameRuleListArgs),

    /// Print the path the rename rules live in
    Path(RenameRulePathArgs),

    /// Remove rule by id or --all
    Remove(RenameRuleRemoveArgs),
}

impl RenameRuleCommand {
    pub fn invoke(self) -> eyre::Result<()> {
        match self {
            RenameRuleCommand::Add(a) => a.invoke(),
            RenameRuleCommand::List(a) => a.invoke(),
            RenameRuleCommand::Path(a) => a.invoke(),
            RenameRuleCommand::Remove(a) => a.invoke(),
        }
    }
}

impl ToArgs for RenameRuleCommand {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        match self {
            RenameRuleCommand::Add(a) => {
                args.push("add".into());
                args.extend(a.to_args());
            }
            RenameRuleCommand::List(a) => {
                args.push("list".into());
                args.extend(a.to_args());
            }
            RenameRuleCommand::Path(a) => {
                args.push("path".into());
                args.extend(a.to_args());
            }
            RenameRuleCommand::Remove(a) => {
                args.push("remove".into());
                args.extend(a.to_args());
            }
        }
        args
    }
}

#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct RenameRuleAddArgs {
    /// Find pattern (regex)
    pub find: String,
    /// Replacement string (optional)
    #[clap(default_value = "")]
    pub replace: String,
    /// Only apply when name is too long (longer than max name length)
    #[clap(long = "only-when-too-long")]
    pub only_when_too_long: bool,
    /// Case-sensitive match (default is case-insensitive)
    #[clap(long = "case-sensitive")]
    pub case_sensitive: bool,
    /// Create the rule in a disabled state
    #[clap(long = "disabled")]
    pub disabled: bool,
}

impl RenameRuleAddArgs {
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

impl ToArgs for RenameRuleAddArgs {
    fn to_args(&self) -> Vec<OsString> {
        let mut rtn = vec![
            OsString::from(self.find.clone()),
            OsString::from(self.replace.clone()),
        ];
        if self.only_when_too_long {
            rtn.push("--only-when-too-long".into());
        }
        if self.case_sensitive {
            rtn.push("--case-sensitive".into());
        }
        if self.disabled {
            rtn.push("--disabled".into());
        }
        rtn
    }
}

#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct RenameRuleListArgs {}

impl RenameRuleListArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        let listed = list_rules(&APP_HOME)?;
        info!("Found {} rename rules", listed.len());
        for (_i, rule) in listed {
            println!("{}: {}", rule.id, rule);
        }
        Ok(())
    }
}

impl ToArgs for RenameRuleListArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![]
    }
}

#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct RenameRuleRemoveArgs {
    /// Remove all rules
    #[clap(long)]
    pub all: bool,
    /// Rule id (UUID). If omitted and --all is specified, removes all rules.
    pub id: Option<String>,
}

impl RenameRuleRemoveArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        let listed = list_rules(&APP_HOME)?;
        if self.all {
            if self.id.is_some() {
                println!("Cannot specify an id with --all");
                return Ok(());
            }
            let mut removed = 0usize;
            for (_i, rule) in listed {
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

impl ToArgs for RenameRuleRemoveArgs {
    fn to_args(&self) -> Vec<OsString> {
        let mut rtn = Vec::new();
        if self.all {
            rtn.push("--all".into());
        }
        if let Some(id) = &self.id {
            rtn.push(OsString::from(id.clone()));
        }
        rtn
    }
}

#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct RenameRulePathArgs {}

impl RenameRulePathArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        let p = crate::rename_rules::rules_dir(&APP_HOME)?;
        println!("{}", p.display());
        Ok(())
    }
}

impl ToArgs for RenameRulePathArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![]
    }
}
