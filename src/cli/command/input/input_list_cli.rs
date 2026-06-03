use crate::app_home::APP_HOME;
use crate::inputs;
use arbitrary::Arbitrary;
use facet::Facet;

#[derive(Facet, Arbitrary, Clone, PartialEq, Debug)]
#[facet(rename_all = "kebab-case")]
pub struct InputListArgs {}

impl InputListArgs {
    /// # Errors
    ///
    /// Returns an error if loading the input paths fails.
    pub fn invoke(self) -> eyre::Result<()> {
        let list = inputs::load_inputs(&APP_HOME)?;
        for p in list {
            println!("{}", p.display());
        }
        Ok(())
    }
}
