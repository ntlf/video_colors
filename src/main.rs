use clap::Parser;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;
use tracing::{debug, info, Level};
use video_colors::{extract_colors, write_colors_to_file};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Input video file to operate on
    input: PathBuf,

    /// Optional output file, defaults to input file name with `.json` extension
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

fn main() -> Result<(), Box<dyn Error>> {
    let timer = Instant::now();
    let args = Cli::parse();

    tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_max_level(match args.debug {
            0 => Level::INFO,
            1 => Level::DEBUG,
            _ => Level::TRACE,
        })
        .with_thread_ids(true)
        .init();

    info!("Extracting colors from {}", args.input.display());

    let input = args.input.into_os_string().into_string().unwrap();
    let output = args
        .output
        .unwrap_or_else(|| PathBuf::from(format!("{}.json", input)))
        .into_os_string()
        .into_string()
        .unwrap();

    debug!(input, output, debug = args.debug);

    let colors = extract_colors(&input)?;

    write_colors_to_file(&colors, &output);

    info!("Done in {:.2?}", timer.elapsed());

    Ok(())
}
