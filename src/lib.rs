use opencv::core::no_array;
use opencv::imgproc::{self, COLOR_BGR2RGB};
use opencv::prelude::*;
use opencv::videoio::{
    VideoCapture, CAP_ANY, CAP_PROP_FPS, CAP_PROP_FRAME_COUNT, CAP_PROP_FRAME_HEIGHT,
    CAP_PROP_FRAME_WIDTH, CAP_PROP_POS_FRAMES,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tracing::{debug, trace};

#[derive(Debug)]
struct VideoStats {
    fps: i32,
    frame_count: i32,
    #[allow(dead_code)]
    width: i32,
    #[allow(dead_code)]
    height: i32,
}

pub fn extract_colors(input: &str) -> Result<Vec<[u8; 3]>, Box<dyn Error>> {
    let video = VideoCapture::from_file(input, CAP_ANY)?;
    let stats = get_stats(&video)?;

    debug!(stats = ?stats);

    let fps = stats.fps;
    let frame_count = stats.frame_count;

    let min_chunk_size = fps * 90;
    let number_of_chunks = std::cmp::min(
        std::thread::available_parallelism().unwrap().get() - 1,
        (frame_count as f64 / min_chunk_size as f64).ceil() as usize,
    );

    debug!(number_of_chunks);

    let chunks = (0..frame_count)
        .collect::<Vec<_>>()
        .par_chunks((frame_count as f64 / number_of_chunks as f64).ceil() as usize)
        .map(|chunk| chunk.to_owned())
        .collect::<Vec<_>>();

    debug!(chunks = ?(chunks.iter().map(|chunk| (chunk[0]..=chunk[chunk.len() - 1])).collect::<Vec<_>>()));

    let colors = chunks
        .par_iter()
        .flat_map(|chunk| {
            let mut video = VideoCapture::from_file(input, CAP_ANY).unwrap();
            video.set(CAP_PROP_POS_FRAMES, chunk[0] as f64).unwrap();

            debug!(chunk = ?(chunk[0]..=chunk[chunk.len() - 1]));

            get_colors(&mut video, &stats, chunk).unwrap()
        })
        .collect::<Vec<_>>();

    debug!(
        colors = format!(
            "[{}, ... {}]",
            colors
                .iter()
                .take(3)
                .map(|c| format!("{:?}", c))
                .collect::<Vec<_>>()
                .join(", "),
            colors
                .iter()
                .rev()
                .take(2)
                .rev()
                .map(|c| format!("{:?}", c))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        len = colors.len()
    );

    Ok(colors)
}

fn get_stats(video: &VideoCapture) -> Result<VideoStats, Box<dyn Error>> {
    let fps = video.get(CAP_PROP_FPS)? as i32;
    let frame_count = video.get(CAP_PROP_FRAME_COUNT)? as i32;
    let width = video.get(CAP_PROP_FRAME_WIDTH)? as i32;
    let height = video.get(CAP_PROP_FRAME_HEIGHT)? as i32;

    Ok(VideoStats {
        fps,
        frame_count,
        width,
        height,
    })
}

fn get_colors(
    video: &mut VideoCapture,
    stats: &VideoStats,
    chunk: &[i32],
) -> Result<Vec<[u8; 3]>, Box<dyn Error>> {
    let fps = stats.fps;

    let mut colors = vec![];

    chunk.iter().for_each(|i| {
        if i % fps == 0 {
            let mut frame = Mat::default();
            video.read(&mut frame).unwrap();

            let color = get_mean_color(&frame).unwrap();

            trace!(i, ?color);

            colors.push(color);
        } else {
            video.grab().unwrap();
        }
    });

    Ok(colors)
}

fn get_mean_color(frame: &Mat) -> Result<[u8; 3], Box<dyn Error>> {
    let mut rgb_frame = Mat::default();
    imgproc::cvt_color(&frame, &mut rgb_frame, COLOR_BGR2RGB, 0).unwrap();

    let mean = opencv::core::mean(&rgb_frame, &no_array()).unwrap();

    Ok([mean[0] as u8, mean[1] as u8, mean[2] as u8])
}

pub fn write_colors_to_file(colors: &Vec<[u8; 3]>, path: &str) {
    #[derive(Serialize, Deserialize)]
    struct Json {
        colors: Vec<[u8; 3]>,
    }

    let json = Json {
        colors: colors.to_owned(),
    };

    std::fs::write(path, serde_json::to_string(&json).unwrap()).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_colors() {
        let colors = extract_colors("data/input.mp4").unwrap();

        assert_eq!(colors.len(), 10);
    }
}
