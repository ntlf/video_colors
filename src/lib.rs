use opencv::core::no_array;
use opencv::imgproc::{self, COLOR_BGR2RGB};
use opencv::prelude::*;
use opencv::videoio::{VideoCapture, CAP_PROP_FPS, CAP_PROP_FRAME_COUNT, CAP_PROP_POS_FRAMES};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use threadpool::ThreadPool;

fn get_frame_color(frame: &Mat) -> [u8; 3] {
    let mut rgb_frame = Mat::default();
    imgproc::cvt_color(&frame, &mut rgb_frame, COLOR_BGR2RGB, 0).unwrap();

    let mean = opencv::core::mean(&rgb_frame, &no_array()).unwrap();

    [mean[0] as u8, mean[1] as u8, mean[2] as u8]
}

#[derive(Serialize, Deserialize, Debug)]
struct Json {
    colors: Vec<[u8; 3]>,
}

pub fn write_colors_to_file(colors: &Vec<[u8; 3]>, path: &str) {
    let json = Json {
        colors: colors.to_owned(),
    };

    fs::write(path, serde_json::to_string(&json).unwrap()).unwrap();
}

fn get_video_info(video: &VideoCapture) -> (i32, i32) {
    let fps = video.get(CAP_PROP_FPS).unwrap() as i32;
    let frame_count = video.get(CAP_PROP_FRAME_COUNT).unwrap() as i32;

    (fps, frame_count)
}

pub fn extract_colors(input: &str) -> Vec<[u8; 3]> {
    let timer = Instant::now();

    let mut video = VideoCapture::from_file(input, 0).unwrap();
    let (fps, frame_count) = get_video_info(&video);

    println!("Non-threaded version");
    println!("FPS: {}", fps);
    println!("Frame count: {}", frame_count);

    let mut colors = Vec::new();

    for i in 0..frame_count {
        if i % fps == 0 {
            let mut frame = Mat::default();

            video.read(&mut frame).unwrap();

            let color = get_frame_color(&frame);

            colors.push(color);
        } else {
            video.grab().unwrap();
        }

        print!("{:.2}%\r", i as f64 / frame_count as f64 * 100.0);
    }

    println!("Elapsed time: {:.2?}", timer.elapsed());

    colors
}

/**
 * TODO: This implementation contains a bug
 */
pub fn extract_colors_threaded(input: &str) -> Vec<[u8; 3]> {
    let timer = Instant::now();

    let video = Arc::new(Mutex::new(VideoCapture::from_file(input, 0).unwrap()));
    let (fps, frame_count) = get_video_info(&video.lock().unwrap());

    println!("Threaded version");
    println!("FPS: {}", fps);
    println!("Frame count: {}", frame_count);

    let n_workers = std::thread::available_parallelism().unwrap().get() - 1;
    let pool = ThreadPool::new(n_workers);

    let (tx, rx) = mpsc::channel();

    for i in 0..frame_count {
        let tx = tx.clone();
        let video = video.clone();

        pool.execute(move || {
            if i % fps == 0 {
                let mut frame = Mat::default();

                let mut v = video.lock().unwrap();

                v.set(CAP_PROP_POS_FRAMES, i as f64).unwrap();
                v.read(&mut frame).unwrap();

                drop(v);

                let color = get_frame_color(&frame);

                tx.send((i, color)).unwrap();
            }
        });
    }

    drop(tx);

    let mut messages = vec![];

    for (i, m) in rx.iter().enumerate() {
        print!(
            "{:.2}%\r",
            i as f64 / (frame_count as f64 / fps as f64) * 100.0
        );

        messages.push(m);
    }

    messages.sort_by(|a, b| a.0.cmp(&b.0));

    let colors = messages
        .iter()
        .map(|(_, color)| color.to_owned())
        .collect::<Vec<_>>();

    println!("Elapsed time: {:.2?}", timer.elapsed());

    colors
}

pub fn extract_colors_threaded_chunks(input: &str) -> Vec<[u8; 3]> {
    let timer = Instant::now();

    let video = VideoCapture::from_file(input, 0).unwrap();
    let (fps, frame_count) = get_video_info(&video);

    println!("Threaded-Chunks version");
    println!("FPS: {}", fps);
    println!("Frame count: {}", frame_count);

    let n_workers = std::thread::available_parallelism().unwrap().get() - 1;
    let pool = ThreadPool::new(n_workers);

    let (tx, rx) = mpsc::channel();

    let chunks = (0..frame_count)
        .collect::<Vec<_>>()
        .chunks((frame_count as usize / n_workers) + 1)
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<_>>();

    for chunk in chunks {
        let tx = tx.clone();
        let input = input.to_owned();

        pool.execute(move || {
            let mut video = VideoCapture::from_file(&input, 0).unwrap();
            video.set(CAP_PROP_POS_FRAMES, chunk[0] as f64).unwrap();

            for i in chunk {
                if i % fps == 0 {
                    let mut frame = Mat::default();
                    video.read(&mut frame).unwrap();

                    let color = get_frame_color(&frame);
                    tx.send((i, color)).unwrap();
                } else {
                    video.grab().unwrap();
                }
            }
        });
    }

    drop(tx);

    let mut messages = vec![];

    for (i, m) in rx.iter().enumerate() {
        print!(
            "{:.2}%\r",
            i as f64 / (frame_count as f64 / fps as f64) * 100.0
        );

        messages.push(m);
    }

    messages.sort_by(|a, b| a.0.cmp(&b.0));

    let colors = messages
        .iter()
        .map(|(_, color)| color.to_owned())
        .collect::<Vec<_>>();

    println!("Elapsed time: {:.2?}", timer.elapsed());

    colors
}
