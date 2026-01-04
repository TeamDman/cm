use crate::app_home::APP_HOME;
use crate::cli::to_args::ToArgs;
use crate::rename_rules::RenameRule;
use crate::rename_rules::RenameRuleModifier;
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

    /// Remove rule by index
    Remove(RenameRuleRemoveArgs),
}

impl RenameRuleCommand {
    pub fn invoke(self) -> eyre::Result<()> {
        match self {
            RenameRuleCommand::Add(a) => a.invoke(),
            RenameRuleCommand::List(a) => a.invoke(),
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
    /// Add a when expression like 'len > 50'
    #[clap(long)]
    pub when: Option<String>,
    /// Case-insensitive match
    #[clap(long = "case-insensitive")]
    pub case_insensitive: bool,
}

impl RenameRuleAddArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        let mut modifiers = Vec::new();
        if self.case_insensitive {
            modifiers.push(RenameRuleModifier::CaseInsensitive);
        }
        if let Some(w) = self.when {
            let m = format!("when {}", w);
            modifiers.push(m.parse()?);
        }

        let rule = RenameRule {
            id: Uuid::new_v4(),
            find: self.find,
            replace: self.replace,
            modifiers,
        };
        let id = add_rule(&APP_HOME, &rule)?;
        println!("Added rule {}: {}", id, rule);
        Ok(())
    }
}

impl ToArgs for RenameRuleAddArgs {
    fn to_args(&self) -> Vec<OsString> {
        let mut rtn = vec![
            OsString::from(self.find.clone()),
            OsString::from(self.replace.clone()),
        ];
        if let Some(w) = &self.when {
            rtn.push("--when".into());
            rtn.push(OsString::from(w.clone()));
        }
        if self.case_insensitive {
            rtn.push("--case-insensitive".into());
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
        for (i, rule) in listed {
            println!("{}. {}", i, rule);
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
    /// 1-based rule index
    pub index: usize,
}

impl RenameRuleRemoveArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        let listed = list_rules(&APP_HOME)?;
        if let Some((_i, rule)) = listed.into_iter().find(|(i, _)| *i == self.index) {
            if remove_rule(&APP_HOME, rule.id)? {
                println!("Removed rule {}", self.index);
            } else {
                println!("No rule {}", self.index);
            }
        } else {
            println!("No rule {}", self.index);
        }
        Ok(())
    }
}

impl ToArgs for RenameRuleRemoveArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![OsString::from(format!("{}", self.index))]
    }
}
