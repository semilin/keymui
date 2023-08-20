use km::{self, MetricContext};
use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
use std::ffi::OsStr;
use crate::Keymui;

impl Keymui {
    pub fn import_metrics(&mut self, dir: PathBuf) -> Result<()> {
	let mut added = false;
        for entry in fs::read_dir(dir)? {
	    let entry = entry?;
	    let path = entry.path();
	    println!("{:?}", path.extension());

	    match path.extension() {
		Some(ext) => {
                    println!("{:?}", ext);
                    if ext != OsStr::new("json") {
                        continue
                    }
                    let md: km::MetricData = serde_json::from_str(&fs::read_to_string(path)?)?;
                    let _mc = MetricContext::from(md);
		    added = true;
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
