# keymui
keymui is probably the most flexible layout analyzer available. It is
not ready yet, but I think it's rather exciting and should be useable
fairly soon.

## features / roadmap
- [x] GUI written in [Iced](https://github.com/iced-rs/iced)
- [x] ability to import custom metrics and keyboards from [km_metrics](https://github.com/semilin/km_metrics)
- [x] flexible layouts adaptable to multiple kinds of keyboards
- [x] multiple modes of layout visualization
- [ ] keyboard-driven interface (partially implemented)
- [ ] nstroke list and visualization for any metric
- [ ] fast layout optimization with cached analysis through [keycat](https://github.com/semilin/)
- [ ] tree-based interactive layout development workflow

## installation
I don't really recommend trying keymui in its current state, as it's
not very useful yet. But if you want to get a headstart on exploring
its features, here's how you can.

### build from source
```sh
git clone https://github.com/semilin/keymui
cd keymui
cargo r --release
```

## setup
Again, since keymui is in its early stages, this is an unpleasant
process.

First, clone [km_metrics](https://github.com/semilin/km_metrics). Then
run `python3 main.py` in its directory.

In keymui, run the `import-metrics` command and select the `export`
folder in your `km_metrics` directory.

Then run the `import-corpus` command and select a text file you'd like
to use as a corpus.

Finally, you need layouts. The only way to add them currently is to
put them directly in keymeow's data directory. On Linux, this is
`$XDG_DATA_DIR/keymeow/layouts/` (`XDG_DATA_DIR` defaulting to
`$HOME/.local/share`). On other platforms, it's wherever application
data is supposed to go on that platform. You can find sample layouts
in `keymui/layouts`.

Now that you've gone through that incredibly intuitive and friendly
process, you can now do... well, not much. The most worthwhile thing
to do at this point is check out the codebase of `km_metrics` and try
writing some of your own metrics and keyboards.
