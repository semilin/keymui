use reqwest::blocking::Client;
use serde::Deserialize;
use std::time::Duration;
use std::path::Path;
use std::fs;

#[derive(Deserialize, Debug)]
struct GithubFileData {
    name: String,
    download_url: String,
}

static APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);

pub fn download_files(data_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
	.timeout(Duration::from_secs(10))
	.user_agent(APP_USER_AGENT)
	.build()?;

    download_repo(&client, &data_dir.join("layouts"), "https://api.github.com/repos/semilin/km_layouts/contents/")?;
    download_repo(&client, &data_dir.join("metrics"), "https://api.github.com/repos/semilin/km_metric_data/contents/")?;
    download_repo(&client, &data_dir.join("corpora"), "https://api.github.com/repos/semilin/km_corpora/contents/")?;
    Ok(())
}

pub fn download_repo(client: &Client, directory: &Path, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let resp = client.get(url).send();
    let data = resp?.json::<Vec<GithubFileData>>()?;
    
    for filedata in data {
	println!("downloading {}", filedata.name);
	if let Ok(contents) = client.get(filedata.download_url).send() {
	    let result = fs::write(directory.join(filedata.name), contents.text()?);
	    println!("{:?}", result);
	};
    }
    Ok(())
}
