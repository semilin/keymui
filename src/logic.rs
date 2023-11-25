use crate::layout_display::{ColorStyle, LayoutDisplay};
use crate::Keymui;
use crate::download;
use directories::BaseDirs;
use anyhow::{anyhow, Result};
use kc::Corpus;
use km::{self, MetricContext};
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

pub fn initial_setup() {
    let base_dirs = BaseDirs::new().unwrap();
    let data_dir = base_dirs.data_dir().join("keymeow");
    if data_dir.exists() {
	return;
    }
    fs::create_dir_all(&data_dir.join("layouts")).unwrap();
    fs::create_dir_all(&data_dir.join("corpora")).unwrap();
    fs::create_dir_all(&data_dir.join("metrics")).unwrap();
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
        self.config = serde_json::from_str(&fs::read_to_string(path)?)?;
        Ok(())
    }

    pub fn save_config(&self) -> Result<()> {
        let cdir = self.config_dir();
        let path = cdir.join("config.json");
        let s = serde_json::to_string(&self.config)?;
        fs::write(path, s)?;
        Ok(())
    }

    pub fn load_layouts(&mut self) -> Result<()> {
        let ldir = self.data_dir().join("layouts");
        fs::create_dir_all(&ldir)?;
        for entry in fs::read_dir(ldir)? {
            let path = entry?.path();
            let s = fs::read_to_string(path)?;
            let layout: km::LayoutData = serde_json::from_str(&s)?;
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
            vec![',', '<'],
            vec!['.', '>'],
            vec!['/', '?'],
            vec!['\'', '"'],
            vec![';', ':'],
        ]);
        let mut corpus = Corpus::with_char_list(&mut char_list);

        corpus.add_file(&file)?;

        let text = serde_json::to_string(&corpus)?;
        let mut new_path = cdir.join(file.file_stem().ok_or(anyhow!("couldn't get path stem"))?);
        new_path.set_extension("json");
        write!(File::create(new_path)?, "{}", text)?;
        Ok(())
    }

    pub fn import_metrics(&mut self, dir: PathBuf) -> Result<()> {
        let mut added = false;
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            match path.extension() {
                Some(ext) => {
                    if ext != OsStr::new("json") {
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
                    let s = fs::read_to_string(&path)?;
                    fs::write(newpath, s)?;
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

        for entry in fs::read_dir(cdir)? {
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

    pub fn load_data(&mut self) -> Option<()> {
        let path = self.metric_lists.get(&self.current_metrics.clone()?)?;
        let s = fs::read_to_string(path).ok()?;
        let metrics: km::MetricData = serde_json::from_str(&s).ok()?;

        let corpus = self.current_corpus.clone()?;
        let path = self.corpora.get(&corpus)?;
        let s = fs::read_to_string(path).ok()?;
        let corpus: Corpus = serde_json::from_str(&s).ok()?;

        let mut context = MetricContext::new(
            self.layouts.get(&self.current_layout.clone()?)?,
            metrics,
            corpus,
        )?;

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

        Some(())
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
            for (i, stroke) in ctx.analyzer.data.strokes.iter().enumerate() {
                if stroke
                    .amounts
                    .iter()
                    .any(|m| m.metric == self.nstrokes_metric)
                {
                    self.nstrokes_list.push((
                        i,
                        ctx.analyzer.layouts[0]
                            .nstroke_chars(&ctx.analyzer.data.strokes[i].nstroke)
                            .iter()
                            .map(|c| ctx.analyzer.corpus.uncorpus_unigram(*c))
                            .collect::<String>(),
                    ));
                }
            }
        }
    }

    pub fn sort_nstroke_list(&mut self) {
        if let Some(ctx) = &self.metric_context {
            let an = &ctx.analyzer;
            self.nstrokes_list.sort_by_key(|i| {
                an.layouts[0].frequency(
                    &an.corpus,
                    &an.data.strokes[i.0].nstroke,
                    Some(an.data.metrics[self.nstrokes_metric]),
                )
            });
            self.nstrokes_list.reverse();
        }
    }
}
