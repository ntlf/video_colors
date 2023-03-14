use faiss::Index;
use opencv::core::no_array;
use opencv::imgproc::{self, COLOR_BGR2RGB};
use opencv::prelude::*;
use opencv::videoio::{VideoCapture, CAP_PROP_FPS, CAP_PROP_FRAME_COUNT, CAP_PROP_POS_FRAMES};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;
use tracing::{debug, trace};

#[allow(dead_code)]
enum ColorMode {
    Mean = 0,
    Dominant = 1,
}

fn get_frame_color(frame: &Mat) -> [u8; 3] {
    let mode = ColorMode::Mean;

    match mode {
        ColorMode::Mean => get_mean_color(frame),
        ColorMode::Dominant => get_dominant_color(frame),
    }
}

fn get_mean_color(frame: &Mat) -> [u8; 3] {
    let mut rgb_frame = Mat::default();
    imgproc::cvt_color(&frame, &mut rgb_frame, COLOR_BGR2RGB, 0).unwrap();

    let mean = opencv::core::mean(&rgb_frame, &no_array()).unwrap();

    [mean[0] as u8, mean[1] as u8, mean[2] as u8]
}

fn get_dominant_color(frame: &Mat) -> [u8; 3] {
    let mut rgb_frame = Mat::default();
    imgproc::cvt_color(&frame, &mut rgb_frame, COLOR_BGR2RGB, 0).unwrap();

    let d = rgb_frame
        .reshape(1, rgb_frame.total() as i32 * rgb_frame.channels())
        .unwrap();

    let mut d0 = Mat::default();

    d.convert_to(&mut d0, opencv::core::CV_32F, 1.0, 0.0)
        .unwrap();

    let d_data = d0.data_typed::<f32>().unwrap();

    const K: u32 = 10;

    let mut params = faiss::cluster::ClusteringParameters::new();
    params.set_niter(10);
    params.set_nredo(1);
    params.set_verbose(false);

    let mut kmeans = faiss::cluster::Clustering::new_with_params(3, K, &params).unwrap();
    let mut index = faiss::FlatIndex::new(3, faiss::MetricType::L2).unwrap();
    kmeans.train(d_data, &mut index).unwrap();
    let faiss::index::SearchResult {
        distances: _,
        labels,
    } = index.search(d_data, 1).unwrap();

    let counts = labels
        .par_iter()
        .fold(
            || [0; K as usize],
            |mut acc, l| {
                acc[l.to_native() as usize] += 1;
                acc
            },
        )
        .reduce(
            || [0; K as usize],
            |mut acc1, acc2| {
                for i in 0..K as usize {
                    acc1[i] += acc2[i];
                }
                acc1
            },
        );

    let max = counts
        .par_iter()
        .enumerate()
        .max_by_key(|(_, v)| **v)
        .unwrap();

    let centroids = kmeans.centroids().unwrap();

    let dominant_color = centroids[max.0];

    [
        dominant_color[0] as u8,
        dominant_color[1] as u8,
        dominant_color[2] as u8,
    ]
}

#[derive(Serialize, Deserialize, Debug)]
struct Json {
    colors: Vec<[u8; 3]>,
}

pub fn write_colors_to_file(colors: &Vec<[u8; 3]>, path: &str) {
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
        path
    );

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
    let mut video = VideoCapture::from_file(input, 0).unwrap();

    let (fps, frame_count) = get_video_info(&video);

    debug!(fps, frame_count);

    let mut colors = Vec::new();

    for i in 0..frame_count {
        if i % fps == 0 {
            let mut frame = Mat::default();

            video.read(&mut frame).unwrap();

            let color = get_frame_color(&frame);

            trace!(i, ?color);

            colors.push(color);
        } else {
            video.grab().unwrap();
        }
    }

    colors
}

pub fn extract_colors_threaded(input: &str) -> Vec<[u8; 3]> {
    let video = Arc::new(Mutex::new(VideoCapture::from_file(input, 0).unwrap()));
    let (fps, frame_count) = get_video_info(&video.lock().unwrap());

    debug!(fps, frame_count);

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

                trace!(i, ?color);

                tx.send((i, color)).unwrap();
            }
        });
    }

    drop(tx);

    let mut messages = rx.iter().collect::<Vec<_>>();
    messages.sort_by(|a, b| a.0.cmp(&b.0));

    let colors = messages
        .iter()
        .map(|(_, color)| color.to_owned())
        .collect::<Vec<_>>();

    debug!(colors = ?colors);

    colors
}

pub fn extract_colors_threaded_chunks(input: &str) -> Vec<[u8; 3]> {
    let video = VideoCapture::from_file(input, 0).unwrap();
    let (fps, frame_count) = get_video_info(&video);

    let min_chunk_size = fps * 90;
    let number_of_chunks = std::cmp::min(
        std::thread::available_parallelism().unwrap().get() - 1,
        (frame_count as f64 / min_chunk_size as f64).ceil() as usize,
    );

    debug!(fps, frame_count, number_of_chunks);

    let pool = ThreadPool::new(number_of_chunks);

    let (tx, rx) = mpsc::channel();

    let chunks = (0..frame_count)
        .collect::<Vec<_>>()
        .chunks((frame_count as f64 / number_of_chunks as f64).ceil() as usize)
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<_>>();

    debug!(chunks = ?(chunks.iter().map(|chunk| (chunk[0]..chunk[chunk.len() - 1])).collect::<Vec<_>>()));

    for chunk in chunks {
        let tx = tx.clone();
        let input = input.to_owned();

        pool.execute(move || {
            let mut video = VideoCapture::from_file(&input, 0).unwrap();
            video.set(CAP_PROP_POS_FRAMES, chunk[0] as f64).unwrap();

            debug!(chunk = ?(chunk[0]..chunk[chunk.len() - 1]));

            for i in chunk {
                if i % fps == 0 {
                    let mut frame = Mat::default();
                    video.read(&mut frame).unwrap();

                    let color = get_frame_color(&frame);

                    trace!(i, ?color);

                    tx.send((i, color)).unwrap();
                } else {
                    video.grab().unwrap();
                }
            }
        });
    }

    drop(tx);

    let mut messages = rx.iter().collect::<Vec<_>>();
    messages.sort_by(|a, b| a.0.cmp(&b.0));

    let colors = messages
        .iter()
        .map(|(_, color)| color.to_owned())
        .collect::<Vec<_>>();

    colors
}

pub fn extract_colors_threaded_rayon(input: &str) -> Vec<[u8; 3]> {
    let video = VideoCapture::from_file(input, 0).unwrap();
    let (fps, frame_count) = get_video_info(&video);

    let min_chunk_size = fps * 90;
    let number_of_chunks = std::cmp::min(
        std::thread::available_parallelism().unwrap().get() - 1,
        (frame_count as f64 / min_chunk_size as f64).ceil() as usize,
    );

    debug!(fps, frame_count, number_of_chunks);

    let chunks = (0..frame_count)
        .collect::<Vec<_>>()
        .par_chunks((frame_count as f64 / number_of_chunks as f64).ceil() as usize)
        .map(|chunk| chunk.to_owned())
        .collect::<Vec<_>>();

    debug!(chunks = ?(chunks.iter().map(|chunk| (chunk[0]..chunk[chunk.len() - 1])).collect::<Vec<_>>()));

    let colors = chunks
        .par_iter()
        .flat_map(|chunk| {
            let mut video = VideoCapture::from_file(input, 0).unwrap();
            video.set(CAP_PROP_POS_FRAMES, chunk[0] as f64).unwrap();

            debug!(chunk = ?(chunk[0]..chunk[chunk.len() - 1]));

            let mut chunk_colors = vec![];

            chunk.iter().for_each(|i| {
                if i % fps == 0 {
                    let mut frame = Mat::default();
                    video.read(&mut frame).unwrap();

                    let color = get_frame_color(&frame);

                    trace!(i, ?color);

                    chunk_colors.push(color);
                } else {
                    video.grab().unwrap();
                }
            });

            chunk_colors
        })
        .collect::<Vec<_>>();

    colors
}
