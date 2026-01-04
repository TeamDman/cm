use crate::rename_rules::RenameRuleModifier;
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameRule {
    pub find: String,
    pub replace: String,
    pub modifiers: Vec<RenameRuleModifier>,
}

impl Default for RenameRule {
    fn default() -> Self {
        Self {
            find: String::new(),
            replace: String::new(),
            modifiers: Vec::new(),
        }
    }
}

impl RenameRule {
    /// Serialize rule to file text
    pub fn to_file_text(&self) -> String {
        let mut s = String::new();
        s.push_str(&self.find);
        s.push('\n');
        s.push_str(&self.replace);
        s.push('\n');
        for m in &self.modifiers {
            s.push_str(&m.to_string());
            s.push('\n');
        }
        s
    }

    /// Parse from file text
    pub fn from_file_text(text: &str) -> eyre::Result<Self> {
        let mut lines = text.lines();
        let find = lines.next().unwrap_or("").to_string();
        let replace = lines.next().unwrap_or("").to_string();
        let mut modifiers = Vec::new();
        for l in lines {
            let l = l.trim();
            if l.is_empty() {
                continue;
            }
            let m = l.parse()?;
            modifiers.push(m);
        }
        Ok(RenameRule { find, replace, modifiers })
    }
}

impl fmt::Display for RenameRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\" \"{}\"", self.find, self.replace)
    }
}

impl FromStr for RenameRule {
    type Err = eyre::Report;

    /// Parse a single-line representation "find" "replace" [modifiers...]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Very simple parser for CLI convenience
        let parts: Vec<&str> = s.split('"').collect();
        if parts.len() >= 3 {
            let find = parts[1].to_string();
            let replace = if parts.len() >= 5 { parts[3].to_string() } else { String::new() };
            Ok(RenameRule { find, replace, modifiers: Vec::new() })
        } else {
            Err(eyre::eyre!("Failed to parse rule"))
        }
    }
}