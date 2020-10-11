use crate::errors::Result;
use crate::{
    lang::{component, context},
    utils::OutputFile,
};
use pretty::termcolor::ColorSpec;
use pretty::RcDoc;

/// A backend for FuTIL.
pub trait Backend {
    /// The name of this backend.
    fn name() -> &'static str;
    /// Validate this program for emitting using this backend. Returns an
    /// Err(..) if the program has unexpected constructs.
    fn validate(prog: &context::Context) -> Result<()>;
    /// Transforms the program into a formatted string representing a valid
    /// and write it to `write`.
    fn emit(prog: &context::Context, write: OutputFile) -> Result<()>;
    /// Convience function to validate and emit the program.
    fn run(prog: &context::Context, file: OutputFile) -> Result<()> {
        Self::validate(&prog)?;
        Self::emit(prog, file)
    }
}

/// Represents something that can be transformed in to a RcDoc.
pub trait Emitable {
    fn doc<'a>(
        &self,
        ctx: &context::Context,
        comp: &component::Component,
    ) -> Result<RcDoc<'a, ColorSpec>>;
}
