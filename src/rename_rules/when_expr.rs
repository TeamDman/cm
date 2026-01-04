use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WhenExpr {
    LengthIsGreaterThan(usize),
}

impl fmt::Display for WhenExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WhenExpr::LengthIsGreaterThan(n) => write!(f, "len > {}", n),
        }
    }
}

impl FromStr for WhenExpr {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        // Accept both 'len > N' and 'when len > N'
        let s = if s.to_lowercase().starts_with("when ") { &s[5..] } else { s };
        let parts: Vec<_> = s.split('>').map(|p| p.trim()).collect();
        if parts.len() == 2 && parts[0].to_ascii_lowercase() == "len" {
            let n: usize = parts[1].parse().map_err(|_| eyre::eyre!("Invalid number in when expression"))?;
            return Ok(WhenExpr::LengthIsGreaterThan(n));
        }
        Err(eyre::eyre!("Unsupported when expression: {}", s))
    }
}