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

    download_layouts(&data_dir.join("layouts"), &client)?;
    download_metrics(&data_dir.join("metrics"), &client)?;
    download_corpora(&data_dir.join("corpora"), &client)?;
    Ok(())
}

pub fn download_layouts(layouts_dir: &Path, client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let resp = client.get("https://api.github.com/repos/semilin/km_layouts/contents/").send();
    let data = resp?.json::<Vec<GithubFileData>>()?;
    
    for filedata in data {
	println!("downloading {}", filedata.name);
	if let Ok(layout_json) = client.get(filedata.download_url).send() {
	    let result = fs::write(layouts_dir.join(filedata.name), layout_json.text()?);
	    println!("{:?}", result);
	};
    }
    Ok(())
}

pub fn download_metrics(metrics_dir: &Path, client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let resp = client.get("https://api.github.com/repos/semilin/km_metric_data/contents/").send();
    let data = resp?.json::<Vec<GithubFileData>>()?;

    for filedata in data {
	println!("downloading {}", filedata.name);
	if let Ok(metrics_json) = client.get(filedata.download_url).send() {
	    let result = fs::write(metrics_dir.join(filedata.name), metrics_json.text()?);
	    println!("{:?}", result);
	}
    }
    Ok(())
}

pub fn download_corpora(corpora_dir: &Path, client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let resp = client.get("https://api.github.com/repos/semilin/km_corpora/contents/").send();
    let data = resp?.json::<Vec<GithubFileData>>()?;

    for filedata in data {
	println!("downloading {}", filedata.name);
	if let Ok(corpus_json) = client.get(filedata.download_url).send() {
	    let result = fs::write(corpora_dir.join(filedata.name), corpus_json.text()?);
	    println!("{:?}", result);
	}
    }
    Ok(())
}
