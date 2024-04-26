use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize, Debug)]
struct GithubFileData {
    name: String,
    download_url: String,
}

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

pub fn download_files(data_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    download_repo(
        &data_dir.join("layouts"),
        "https://api.github.com/repos/semilin/km_layouts/contents/",
    )?;
    download_repo(
        &data_dir.join("metrics"),
        "https://api.github.com/repos/semilin/km_metric_data/contents/",
    )?;
    download_repo(
        &data_dir.join("corpora"),
        "https://api.github.com/repos/semilin/km_corpora/contents/",
    )?;
    Ok(())
}

fn get(url: &str) -> minreq::Request {
    minreq::get(url)
        .with_header("User-Agent", APP_USER_AGENT)
        .with_timeout(8)
}

pub fn download_repo(directory: &Path, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let resp = get(url).send();
    let data = resp?.json::<Vec<GithubFileData>>()?;

    for filedata in data {
        println!("downloading {}", filedata.name);
        if let Ok(contents) = get(&filedata.download_url).send() {
            let result = fs::write(directory.join(filedata.name), contents.as_bytes());
            println!("{:?}", result);
        };
    }
    Ok(())
}
