use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameRule {
    pub id: Uuid,
    pub find: String,
    pub replace: String,
    pub enabled: bool,
    pub case_sensitive: bool,
    pub only_when_name_too_long: bool,
}

impl Default for RenameRule {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            find: String::new(),
            replace: String::new(),
            enabled: true,
            case_sensitive: false,
            only_when_name_too_long: false,
        }
    }
}

impl RenameRule {
    /// Serialize rule to file text (v2 format)
    #[must_use]
    pub fn to_file_text(&self) -> String {
        let mut s = String::new();
        s.push_str(&self.find);
        s.push('\n');
        s.push_str(&self.replace);
        s.push('\n');
        if !self.enabled {
            s.push_str("disabled\n");
        }
        if self.case_sensitive {
            s.push_str("case-sensitive\n");
        }
        if self.only_when_name_too_long {
            s.push_str("only-when-too-long\n");
        }
        s
    }

    /// Parse from file text (v2 format, also accepts legacy v1 format)
    ///
    /// # Errors
    ///
    /// Returns an error if the text format is invalid.
    pub fn from_file_text(text: &str) -> eyre::Result<Self> {
        let mut lines = text.lines();
        let find = lines.next().unwrap_or("").to_string();
        let replace = lines.next().unwrap_or("").to_string();

        let mut enabled = true;
        let mut case_sensitive = false;
        let mut only_when_name_too_long = false;

        for line in lines {
            let l = line.trim().to_ascii_lowercase();
            if l.is_empty() {
                continue;
            }
            // v2 format
            if l == "disabled" {
                enabled = false;
            } else if l == "case-sensitive" {
                case_sensitive = true;
            } else if l == "only-when-too-long" {
                only_when_name_too_long = true;
            }
            // Legacy v1 format compatibility
            else if l == "case-insensitive" || l == "case insensitive" {
                case_sensitive = false; // already default
            } else if l == "always" {
                only_when_name_too_long = false; // already default
            } else if l.starts_with("when ") || l.starts_with("len") {
                // Legacy "when len > N" - treat as only_when_name_too_long
                only_when_name_too_long = true;
            }
        }

        Ok(RenameRule {
            id: Uuid::new_v4(),
            find,
            replace,
            enabled,
            case_sensitive,
            only_when_name_too_long,
        })
    }

    /// Apply rule to a file name. Returns `Some(new_name)` if applied and changed, otherwise None.
    #[must_use]
    pub fn apply(&self, name: &str, max_name_length: usize) -> Option<String> {
        if !self.enabled || self.find.is_empty() {
            return None;
        }

        // Check if rule only applies when name is too long
        if self.only_when_name_too_long && name.len() <= max_name_length {
            return None;
        }

        let mut builder = regex::RegexBuilder::new(&self.find);
        if !self.case_sensitive {
            builder.case_insensitive(true);
        }

        let Ok(re) = builder.build() else { return None };

        let replaced = re.replace_all(name, &self.replace).to_string();

        if replaced == name {
            None
        } else {
            Some(replaced)
        }
    }
}

impl fmt::Display for RenameRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\" \"{}\"", self.find, self.replace)
    }
}

impl FromStr for RenameRule {
    type Err = eyre::Report;

    /// Parse a single-line representation "find" "replace" [flags...]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('"').collect();
        if parts.len() >= 5 {
            let find = parts[1].to_string();
            let replace = parts[3].to_string();
            let rest = parts[4..].join("").to_ascii_lowercase();
            let enabled = !rest.contains("disabled");
            let case_sensitive = rest.contains("case-sensitive");
            let only_when_name_too_long = rest.contains("only-when-too-long");
            Ok(RenameRule {
                id: Uuid::new_v4(),
                find,
                replace,
                enabled,
                case_sensitive,
                only_when_name_too_long,
            })
        } else {
            Err(eyre::eyre!("Invalid rule format: {}", s))
        }
    }
}
