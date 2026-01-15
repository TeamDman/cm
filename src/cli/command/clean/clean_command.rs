use crate::cache::CACHE_HOME;
use crate::cache::clean_cache;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;

/// Clean cached API responses
#[derive(Args, Arbitrary, Clone, PartialEq, Debug)]
pub struct CleanArgs {
    /// Show what would be cleaned without actually deleting
    #[clap(long)]
    pub dry_run: bool,
}

impl CleanArgs {
    /// # Errors
    ///
    /// Returns an error if there are issues accessing or cleaning the cache directory.
    pub fn invoke(self) -> eyre::Result<()> {
        let cache_dir = CACHE_HOME.api_responses_dir();

        if self.dry_run {
            if !cache_dir.exists() {
                println!("Cache directory does not exist: {}", cache_dir.display());
                return Ok(());
            }

            let mut count = 0;
            for entry in std::fs::read_dir(&cache_dir)? {
                let entry = entry?;
                if entry.path().is_dir() {
                    count += 1;
                    println!("Would remove: {}", entry.path().display());
                }
            }
            println!("\nWould remove {count} cache entries");
        } else {
            let result = clean_cache()?;
            println!(
                "Cleaned {} cache entries from {}",
                result.entries_removed,
                cache_dir.display()
            );
        }

        Ok(())
    }
}

impl ToArgs for CleanArgs {
    fn to_args(&self) -> Vec<OsString> {
        let mut rtn = vec![];
        if self.dry_run {
            rtn.push(OsString::from("--dry-run"));
        }
        rtn
    }
}
