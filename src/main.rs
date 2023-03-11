use clap::Parser;
use std::path::PathBuf;
use std::time::Instant;
use tracing::{debug, info, Level};
use video_colors::{
    extract_colors, extract_colors_threaded, extract_colors_threaded_chunks, write_colors_to_file,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Input video file to operate on
    input: PathBuf,

    /// Optional output file, defaults to input file name with `.json` extension
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Use threaded version of color extraction
    #[arg(short, long)]
    threaded: bool,

    /// Use threaded version of color extraction with chunks
    #[arg(short, long)]
    chunks: bool,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

fn main() {
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

    debug!(
        input,
        output,
        threaded = args.threaded,
        chunks = args.chunks
    );

    let colors = if args.threaded {
        if args.chunks {
            extract_colors_threaded_chunks(&input)
        } else {
            extract_colors_threaded(&input)
        }
    } else {
        extract_colors(&input)
    };

    write_colors_to_file(&colors, &output);

    info!("Done in {:.2?}", timer.elapsed());
}
