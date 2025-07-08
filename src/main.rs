use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::anyhow;
use futures_util::StreamExt;
use reqwest::Url;

#[tokio::main]
async fn main() {
    main2().await.unwrap();
}

struct LibreOffice;

impl LibreOffice {
    fn url(&self) -> String {
        "https://download.documentfoundation.org/libreoffice/portable/25.2.3/LibreOfficePortable_25.2.3_MultilingualStandard.paf.exe".to_string()
    }
    async fn download(&self, dir: &Path) -> Result<PathBuf, anyhow::Error> {
        let u = self.url();
        let parsed = Url::parse(&u)?;
        let filename = parsed
            .path_segments()
            .ok_or(anyhow!(""))?
            .next_back()
            .ok_or(anyhow!(""))?;
        let local_file_path = dir.join(filename);
        let local_file = File::create(&local_file_path)?;
        let mut local_file = BufWriter::new(local_file);
        let mut stream = reqwest::get(u).await?.bytes_stream();
        while let Some(Ok(x)) = stream.next().await {
            local_file.write_all(x.as_ref())?;
        }
        Ok(local_file_path)
    }
    fn extract(&self, archive_path: &Path, target_dir: &Path) -> Result<(), anyhow::Error> {
        Command::new(archive_path)
            .arg("/S")
            .arg(format!("/D={}", target_dir.to_string_lossy()))
            .output()?;
        Ok(())
    }
}

async fn main2() -> Result<(), anyhow::Error> {
    let l = LibreOffice;
    let arch = &l.download(Path::new("/tmp/")).await?;
    Ok(())
}
