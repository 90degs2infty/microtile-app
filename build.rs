// The following lints are disabled (=`allow`ed) for the moment being. Turn them
// active once you start documenting the public interface properly.
#![allow(
    missing_docs,
    rustdoc::missing_crate_level_docs,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

use std::error::Error;
use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    // Emit the instructions
    EmitBuilder::builder()
        .git_describe(true, true, None)
        .emit()?;
    Ok(())
}
