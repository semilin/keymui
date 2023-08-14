use km::{self, MetricContext};
use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::ffi::OsStr;
use crate::Keymui;

impl Keymui {
    pub fn import_metrics(&mut self, dir: PathBuf) -> Result<()> {
        for entry in fs::read_dir(dir).unwrap() {
	    let entry = entry.unwrap();
	    let path = entry.path();
	    println!("{:?}", path.extension());

	    match path.extension() {
		Some(ext) => {
                    println!("{:?}", ext);
                    if ext != OsStr::new("json") {
                        continue
                    }
                    let md: km::MetricData = serde_json::from_str(&fs::read_to_string(path)?)?;
                    println!("success");
		}
                None => continue
	    };
	}

        Ok(())
    }
}
