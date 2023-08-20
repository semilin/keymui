use km::{self, MetricContext};
use anyhow::{anyhow, Result};
use std::fs::{self, File};
use std::path::PathBuf;
use std::ffi::OsStr;
use crate::Keymui;

impl Keymui {
    pub fn load_metrics(&mut self) -> Result<()> {
	let mdir = self.base_dirs.data_dir().join("keymeow").join("metrics");
	fs::create_dir_all(mdir.clone())?;
	self.import_metrics(mdir, false)
    }
    
    pub fn import_metrics(&mut self, dir: PathBuf, save: bool) -> Result<()> {
	let mut added = false;
        for entry in fs::read_dir(dir)? {
	    let entry = entry?;
	    let path = entry.path();
	    println!("{:?}", path.extension());

	    match path.extension() {
		Some(ext) => {
		    if ext != OsStr::new("json") {
                        continue
                    }
		    let name = path.file_stem().ok_or(anyhow!("couldn't get path stem"))?.to_string_lossy().to_string();
		    let s = fs::read_to_string(path.clone())?;
		    let md: km::MetricData = serde_json::from_str(&s)?;
                    let mc = MetricContext::from(md);
		    self.metric_contexts.insert(name, mc);
		    added = true;

		    if save {
			let mdir = self.base_dirs.data_dir().join("keymeow").join("metrics");
			let newpath = mdir.join(path.file_name().ok_or(anyhow!("couldn't get filename"))?);
		    	fs::write(newpath, s)?;
		    }
		}
                None => continue
	    };
	}

        if added {
	    Ok(())
	} else {
	    Err(anyhow!("directory contained no metric files"))
	}
    }
}
