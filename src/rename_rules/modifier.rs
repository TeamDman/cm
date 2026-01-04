use crate::rename_rules::when_expr::WhenExpr;
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenameRuleModifier {
    Always,
    CaseInsensitive,
    When(WhenExpr),
}

impl fmt::Display for RenameRuleModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenameRuleModifier::Always => write!(f, "always"),
            RenameRuleModifier::CaseInsensitive => write!(f, "case-insensitive"),
            RenameRuleModifier::When(expr) => write!(f, "when {}", expr),
        }
    }
}

impl FromStr for RenameRuleModifier {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let low = s.trim().to_ascii_lowercase();
        if low == "always" {
            return Ok(RenameRuleModifier::Always);
        }
        if low == "case-insensitive" || low == "case insensitive" {
            return Ok(RenameRuleModifier::CaseInsensitive);
        }
        if low.starts_with("when ") {
            let rest = s.trim()[5..].trim();
            let expr = rest.parse()?;
            return Ok(RenameRuleModifier::When(expr));
        }
        // Try parsing as WhenExpr directly
        if low.starts_with("len") {
            let expr = s.trim().parse()?;
            return Ok(RenameRuleModifier::When(expr));
        }
        Err(eyre::eyre!("Unknown modifier: {}", s))
    }
}