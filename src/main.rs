use std::env;

use video_colors::{
    extract_colors, extract_colors_threaded, extract_colors_threaded_chunks, write_colors_to_file,
};

fn main() {
    let input = env::args().nth(1).unwrap();
    let threaded = env::args().any(|arg| arg == "--threaded");
    let chunks = env::args().any(|arg| arg == "--chunks");

    let colors = if threaded {
        if chunks {
            extract_colors_threaded_chunks(&input)
        } else {
            extract_colors_threaded(&input)
        }
    } else {
        extract_colors(&input)
    };

    write_colors_to_file(&colors, format!("{}.json", &input).as_str());
}
