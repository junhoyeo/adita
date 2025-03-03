use clap::Parser;

mod error;
use error::Result;

mod fragment;

mod generator;

mod processor;
use processor::AbiProcessor;

#[derive(Parser, Debug)]
#[command(version, about = "ABI to TypeScript code generator")]
struct Args {
    /// Source directory containing JSON ABI files
    #[arg(short, long, required = true)]
    source: String,

    /// Output directory for TypeScript files
    #[arg(short, long, default_value = "./abis")]
    out_dir: String,
}

fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Setup processor
    let mut processor = AbiProcessor::new(&args.out_dir);

    // Process source files
    let source_pattern = format!("{}/**/*.json", args.source);
    processor.collect_abi_files(&source_pattern)?;

    // Generate TypeScript files
    processor.generate_typescript_files()?;

    Ok(())
}
