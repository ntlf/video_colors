# video_colors

This is a simple experiment to extract dominant colors from a video file with OpenCV and Rust.

## Usage

```sh
$ cargo run -- --help # or you can use the binary file after building the project

Usage: video_colors [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Input video file to operate on

Options:
  -o, --output <FILE>  Optional output file, defaults to input file name with `.json` extension
  -d, --debug...       Turn debugging information on
  -h, --help           Print help
  -V, --version        Print version
```

## Image generation

Turn your extracted colors into a 4k image with the help of a simple Node.js script.

```sh
yarn start <INPUT.json>
```

![Example](./data/example.png)
