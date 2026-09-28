# ploton

Render [ECharts](https://echarts.apache.org) options and CSV datasets in a native window.

![demo](https://github.com/nlkli/assetsrepo/blob/main/ploton.demo/first.gif)

## Usage

```
ploton [OPTIONS] <FILE>...

  <FILE>         ECharts option (JS object literal or JSON) or a .csv dataset
  -w, --watch    Reload the next <FILE> whenever it changes
  -h, --help     Print help
  -V, --version  Print version
```

```sh
ploton -w option.js -w data.csv  # live reload
```

- `.csv` files become a dataset; the first row is the header.
- Multiple files are applied in order, later ones merge into earlier ones.

## Build

Requires Rust 1.85+ and Node.js/npm. The build script builds the web frontend
(`web/`) and embeds it into the binary.

```sh
cargo build --release
```

On Linux, [wry](https://github.com/tauri-apps/wry) needs GTK 3 and WebKitGTK 4.1
(e.g. `libgtk-3-dev libwebkit2gtk-4.1-dev`).

