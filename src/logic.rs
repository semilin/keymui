use crate::layout_display::{ColorStyle, LayoutDisplay};
use crate::Keymui;
use crate::{download, NstrokeSortMethod};
use color_eyre::eyre::{anyhow, Context, ContextCompat, Result};
use directories::BaseDirs;
use kc::Corpus;
use km::{self, MetricContext};
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

pub fn initial_setup() {
    let base_dirs = BaseDirs::new().unwrap();
    let data_dir = base_dirs.data_dir().join("keymeow");
    if data_dir.exists() {
        return;
    }
    fs::create_dir_all(data_dir.join("layouts")).unwrap();
    fs::create_dir_all(data_dir.join("corpora")).unwrap();
    fs::create_dir_all(data_dir.join("metrics")).unwrap();
    let _ = download::download_files(&data_dir);
}

impl Keymui {
    pub fn config_dir(&self) -> PathBuf {
        self.base_dirs.config_dir().join("keymeow")
    }

    pub fn data_dir(&self) -> PathBuf {
        self.base_dirs.data_dir().join("keymeow")
    }

    pub fn load_config(&mut self) -> Result<()> {
        let cdir = self.config_dir();
        fs::create_dir_all(&cdir)?;
        let path = cdir.join("config.json");
        self.config = serde_json::from_str(&fs::read_to_string(&path).context(format!(
            "couldn't read config file from path {}",
            &path.display()
        ))?)
        .context("couldn't parse config file")?;
        Ok(())
    }

    pub fn save_config(&self) -> Result<()> {
        let cdir = self.config_dir();
        let path = cdir.join("config.json");
        let s = serde_json::to_string_pretty(&self.config)?;
        fs::write(&path, s)
            .context(format!("couldn't write config file to {}", &path.display()))?;
        Ok(())
    }

    pub fn load_layouts(&mut self) -> Result<()> {
        let ldir = self.data_dir().join("layouts");
        fs::create_dir_all(&ldir)?;
        for entry in fs::read_dir(ldir).context("couldn't read layouts directory")? {
            let path = entry?.path();
            let s = fs::read_to_string(&path)
                .with_context(|| format!("couldn't read file {}", &path.display()))?;
            let layout: km::LayoutData = serde_json::from_str(&s)
                .with_context(|| format!("couldn't parse layout file {}", &path.display()))?;
            self.layouts
                .insert(layout.name.clone().to_lowercase().replace(' ', "-"), layout);
        }
        Ok(())
    }

    pub fn import_corpus(&mut self, file: PathBuf) -> Result<()> {
        let cdir = self.data_dir().join("corpora");
        fs::create_dir_all(&cdir)?;

        // TODO generalize
        let mut char_list = "abcdefghijklmnopqrstuvwxyz"
            .chars()
            .map(|c| vec![c, c.to_uppercase().next().unwrap()])
            .collect::<Vec<Vec<char>>>();
        char_list.extend(vec![
            vec![' '],
            vec![',', '<'],
            vec!['.', '>'],
            vec!['/', '?'],
            vec!['\'', '"'],
            vec![';', ':'],
            vec!['1', '!'],
            vec!['2', '@'],
            vec!['3', '#'],
            vec!['4', '$'],
            vec!['5', '%'],
            vec!['6', '^'],
            vec!['7', '&'],
            vec!['8', '*'],
            vec!['9', '('],
            vec!['0', ')'],
            vec!['-', '_'],
            vec!['=', '+'],
            vec!['[', '{'],
            vec![']', '}'],
            vec!['\\', '|'],
            vec!['`', '~'],
        ]);
        let mut corpus = Corpus::with_char_list(char_list);

        corpus.add_file(&file)?;

        let bin = rmp_serde::to_vec(&corpus)?;
        let mut new_path = cdir.join(file.file_stem().ok_or(anyhow!("couldn't get path stem"))?);
        new_path.set_extension("corpus");
        fs::write(new_path, bin)?;
        Ok(())
    }

    pub fn import_metrics(&mut self) -> Result<()> {
        let mut added = false;
        let path: &PathBuf = self
            .config
            .metrics_directory
            .as_ref()
            .ok_or(anyhow!("no metrics directory set"))?;
        for entry in fs::read_dir(path).context("couldn't read metrics directory")? {
            let entry = entry?;
            let path = entry.path();

            match path.extension() {
                Some(ext) => {
                    if ext != OsStr::new("metrics") {
                        continue;
                    }
                    let name = &path
                        .file_stem()
                        .ok_or(anyhow!("couldn't get path stem"))?
                        .to_string_lossy()
                        .to_string();
                    self.metric_lists.insert(name.clone(), path.clone());
                    added = true;

                    let mdir = self.base_dirs.data_dir().join("keymeow").join("metrics");
                    let newpath =
                        mdir.join(path.file_name().ok_or(anyhow!("couldn't get filename"))?);
                    let b = fs::read(&path).with_context(|| {
                        format!("couldn't read metrics file {}", &path.display())
                    })?;
                    fs::write(&newpath, b).with_context(|| {
                        format!("couldn't write metrics to {}", &newpath.display())
                    })?;
                }
                None => continue,
            };
        }

        if added {
            Ok(())
        } else {
            Err(anyhow!("directory contained no metric files"))
        }
    }

    pub fn set_corpus_list(&mut self) -> Result<()> {
        let cdir = self.data_dir().join("corpora");
        fs::create_dir_all(&cdir)?;

        for entry in fs::read_dir(cdir).context("couldn't read corpus directory")? {
            let path = entry?.path();
            let name = path
                .file_stem()
                .ok_or(anyhow!("couldn't get path stem"))?
                .to_string_lossy()
                .to_string();
            self.corpora.insert(name, path);
        }
        Ok(())
    }

    pub fn load_data(&mut self) -> Result<()> {
        let path = self
            .metric_lists
            .get(
                &self
                    .current_metrics
                    .clone()
                    .context("no metrics selected")?,
            )
            .context("metric data doesn't exist")?;
        let b = fs::read(path).context("couldn't read metrics file")?;
        let metrics: km::MetricData =
            rmp_serde::from_slice(&b).context("couldn't deserialize metrics")?;

        let corpus = self.current_corpus.clone().context("no corpus selected")?;
        let path = self.corpora.get(&corpus).context("corpus doesn't exist")?;
        let b = fs::read(path).context("couldn't read corpus file")?;
        let corpus: Corpus = rmp_serde::from_slice(&b).context("couldn't deserialize corpus")?;

        let mut context = MetricContext::new(
            self.layouts
                .get(&self.current_layout.clone().context("no layout selected")?)
                .context("layout doesn't exist")?,
            metrics,
            corpus,
        )
        .context("couldn't create metric context from selection")?;

        self.layout_stats.clear();
        self.layout_stats
            .resize(context.analyzer.data.metrics.len(), 0.0);
        context
            .analyzer
            .recalc_stats(&mut self.layout_stats, &context.layout);

        context.keyboard.process_combo_indexes();

        self.keyboard_size = context.keyboard.keys.map.iter().flatten().count();

        self.layout_display = Some(LayoutDisplay::new(
            &context,
            ColorStyle::Frequency,
            self.nstrokes_metric,
        ));
        self.metric_context = Some(context);

        self.set_nstroke_list();
        self.sort_nstroke_list();

        Ok(())
    }

    pub fn set_metric_list(&mut self) -> Result<()> {
        let mdir = self.data_dir().join("metrics");
        fs::create_dir_all(&mdir)?;

        for entry in fs::read_dir(mdir)? {
            let path = entry?.path();
            let name = path
                .file_stem()
                .ok_or(anyhow!("couldn't get path stem"))?
                .to_string_lossy()
                .to_string();
            self.metric_lists.insert(name, path);
        }
        Ok(())
    }

    pub fn set_nstroke_list(&mut self) {
        if let Some(ctx) = &self.metric_context {
            self.nstrokes_list = Vec::with_capacity(ctx.analyzer.data.strokes.len() / 3);
            let totals = ctx.layout.totals(&ctx.analyzer.corpus);
            for (i, stroke) in ctx.analyzer.data.strokes.iter().enumerate() {
                let amount = stroke
                    .amounts
                    .iter()
                    .find(|m| m.metric == self.nstrokes_metric);
                if let Some(amt) = amount {
                    let count = ctx.layout.frequency(
                        &ctx.analyzer.corpus,
                        &stroke.nstroke,
                        Some(ctx.analyzer.data.metrics[self.nstrokes_metric]),
                    );
                    let freq_display =
                        totals.percentage(count as f32, ctx.analyzer.data.metrics[amt.metric]);
                    self.nstrokes_list.push((
                        i,
                        ctx.layout
                            .nstroke_chars(&stroke.nstroke)
                            .iter()
                            .map(|c| ctx.analyzer.corpus.uncorpus_unigram(*c))
                            .collect::<String>(),
                        freq_display,
                        amt.amount,
                    ));
                }
            }
        }
    }

    pub fn sort_nstroke_list(&mut self) {
        if let Some(ctx) = &self.metric_context {
            let method = self
                .config
                .metric_display_styles
                .get(&ctx.metrics[self.nstrokes_metric].short)
                .map(|x| x.nstroke_sort_method)
                .unwrap_or_default();
            self.nstrokes_list.sort_by(|a, b| match method {
                NstrokeSortMethod::Frequency => a.2.partial_cmp(&b.2).unwrap(),
                NstrokeSortMethod::Value => a.3.partial_cmp(&b.3).unwrap(),
            });
            self.nstrokes_list.reverse();
        }
    }
}
