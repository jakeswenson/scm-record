use clap::Parser;
use scm_diff_editor::{run, Opts, Result};

pub fn main() -> Result<()> {
    let opts = Opts::parse();

    // Initialize tracing if verbose flag is set
    if opts.verbose {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::DEBUG.into()))
            .with_target(true)
            .with_line_number(true)
            .init();
        eprintln!("Verbose logging enabled");
    }

    run(opts)?;
    Ok(())
}
