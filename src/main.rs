use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::exit;
use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::hash::{compute_hash, get_jar_contents};

/// Simple program to extract the required fields for instancesync
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// the folder where all of the mods reside, ignores non .jar files
    #[arg(short, long)]
    input: PathBuf,

    /// the output mod list .json location
    #[arg(short, long)]
    output: PathBuf,
}

mod hash;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("opening mods");

    let hashes = walkdir::WalkDir::new(&args.input).max_depth(1).min_depth(1).into_iter().filter_map(|i| {
        i.ok()
    }).filter(|f| {
        f.path().extension() == Some("jar".as_ref())
    }).map(|i| {
        let buffer = get_jar_contents(i
            .path()
            .to_str()
            .unwrap_or_else(|| exit_with_error("failed converting jar file path to a string")));

        let output = (i
             .path()
             .file_name()
             .unwrap_or_else(|| exit_with_error("failed getting mod filename"))
             .to_str()
             .unwrap_or_else(|| exit_with_error("failed converting mod filename to string"))
             .to_string(), compute_hash(&buffer));

        println!("hashed {} with hash {}", output.0, output.1);

        output
    })
        .collect::<Vec<_>>();

    let client = Client::new();

    let mut instance = Instance {
        installed_addons: vec![],
        cached_scans: vec![],
    };

    for (file_name, hash) in hashes {
        let res = client
            .post("https://api.curseforge.com/v1/fingerprints/432")
            .body(format!("{{\"fingerprints\": [{}]}}", hash))
            .header("x-api-key", "$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .send()
            .await
            .unwrap_or_else(|err| exit_with_error(&format!("failed getting fingerprint: {}", err)))
            .text()
            .await
            .unwrap_or_else(|err| exit_with_error(&format!("failed getting data of mod: {}", err)));

        let value: Value = serde_json::from_str(&res).unwrap();

        let res = serde_json::to_string_pretty(&value).unwrap();

        let res: FingerprintResponse = serde_json::from_str(&res)
            .unwrap_or_else(|err| exit_with_error(&format!("failed serializing mod data response: {} \n {}", err, &file_name)));

        let MatchFile { download_url } = &res.data.exactMatches[0].file;

        println!("{} fingerprint returned with url {}", file_name, download_url);


        instance.installed_addons.push(Addon{
            installed_file: AddonFile {
                file_name_on_disk: file_name,
                download_url: download_url.to_string()
            }
        })

    }

    instance.installed_addons.sort_by(|a,b| {
        a.installed_file.file_name_on_disk.cmp(&b.installed_file.file_name_on_disk)
    });

    let output = serde_json::to_string_pretty(&instance).unwrap_or_else(|err| exit_with_error(&format!("failed serializing output: {}", err)));


    println!("writing to output file");
    // create and or wipe file
    std::fs::write(&args.output, output).unwrap_or_else(|err| exit_with_error(&format!("failed writing to output file: {}", err)));

}

fn exit_with_error(msg: &str) -> ! {
    eprintln!("{}", msg);
    exit(1);
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FingerprintResponse {
    data: Data
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Data {
    exactMatches: Vec<Match>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Match {
    file: MatchFile
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MatchFile {
    #[serde(rename = "downloadUrl")]
    download_url: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Instance {
    #[serde(rename = "installedAddons")]
    installed_addons: Vec<Addon>,
    #[serde(rename = "cachedScans")]
    cached_scans: Vec<Scan>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Addon {
    #[serde(rename = "installedFile")]
    installed_file: AddonFile,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AddonFile {
    #[serde(rename = "fileNameOnDisk")]
    file_name_on_disk: String,
    #[serde(rename = "downloadUrl")]
    download_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Scan {
    #[serde(rename = "folderName")]
    folder_name: String
}