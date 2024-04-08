# keymui
keymui is probably the most flexible layout analyzer available. It is
not ready yet, but I think it's rather exciting and should be useable
fairly soon.

## features / roadmap
- [x] Cross platform GUI written in [Iced](https://github.com/iced-rs/iced)
  - [ ] Web version
- [x] ability to import custom metrics and keyboards from [km_metrics](https://github.com/semilin/km_metrics)
- [x] flexible layouts adaptable to multiple kinds of keyboards
- [x] multiple modes of layout visualization
- [x] nstroke list and visualization for any metric
- [ ] keyboard-driven interface (partially implemented)
- [ ] fast layout optimization with cached analysis through [keycat](https://github.com/semilin/)
- [ ] tree-based interactive layout development workflow

## installation
Keymui is now pretty usable out of the box. Just download the source
and build it. The only dependency is the Rust compiler.

### build from source
```sh
git clone https://github.com/semilin/keymui
cd keymui
cargo r --release
```

## extra setup
### metrics
Analyzing with the default metrics is nice, but Keymui's real killer
feature is the ability to create your own metrics and keyboards. This
requires some additional setup.

First, clone [km_metrics](https://github.com/semilin/km_metrics).

Run `python3 main.py` to export the metrics.

In keymui, run the `set-metrics-directory` command and select the
`export` folder in your `km_metrics` directory.

In order to refresh the metrics, run the `reload-metrics` command in
keymui. You should do this every time you successfully export in
km_metrics.

### layouts
Adding/editing layouts is annoying at the moment because Keymui stores
layouts in a non-obvious place. On Linux, this is
`$XDG_DATA_DIR/keymeow/layouts/` (`XDG_DATA_DIR` defaulting to
`$HOME/.local/share`). On Windows, this is `%APPDATA%\Roaming\keymeow\layouts\`

In the future, a system will be added for storing layouts in a
directory of your choice.

### corpora
Just run the `import-corpus` command and select a text file. 
