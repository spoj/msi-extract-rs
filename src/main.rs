use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Read},
    path::PathBuf,
};

use anyhow::{Context, anyhow};
use cab::Cabinet;
use msi::{Expr, Package, Select};
use msi_extract::add;

struct MsiTrav {
    f_c: HashMap<String, String>,
    c_d: HashMap<String, String>,
    d_p: HashMap<String, String>,
    f_name: HashMap<String, String>,
    d_name: HashMap<String, String>,
}

impl MsiTrav {
    fn new(package: &mut Package<File>) -> Result<Self, anyhow::Error> {
        let mut f_c = HashMap::new();
        let mut c_d = HashMap::new();
        let mut d_p = HashMap::new();
        let mut f_name = HashMap::new();
        let mut d_name = HashMap::new();

        for r in package.select_rows(Select::table("File"))? {
            if let Some(file) = r["File"].as_str().map(|x| x.to_owned())
                && let Some(filename) = r["FileName"].as_str().map(|x| x.to_owned())
            {
                f_name.insert(file, filename);
            };
            if let Some(file) = r["File"].as_str().map(|x| x.to_owned())
                && let Some(component) = r["Component_"].as_str().map(|x| x.to_owned())
            {
                f_c.insert(file, component);
            };
        }

        for r in package.select_rows(Select::table("Component"))? {
            if let Some(comp) = r["Component"].as_str().map(|x| x.to_owned())
                && let Some(dir) = r["Directory_"].as_str().map(|x| x.to_owned())
            {
                c_d.insert(comp, dir);
            };
        }
        for r in package.select_rows(Select::table("Directory"))? {
            if let Some(dir) = r["Directory"].as_str().map(|x| x.to_owned())
                && let Some(dirpar) = r["Directory_Parent"].as_str().map(|x| x.to_owned())
            {
                d_p.insert(dir, dirpar);
            };
            if let Some(dir) = r["Directory"].as_str().map(|x| x.to_owned())
                && let Some(name) = r["DefaultDir"].as_str().map(|x| match x.split_once('|') {
                    Some((_a, b)) => match b.split_once(':') {
                        Some((c, _d)) => c.to_owned(),
                        None => b.to_owned(),
                    },
                    None => x.to_owned(),
                })
            {
                d_name.insert(dir, name);
            };
        }

        Ok(Self {
            f_c,
            c_d,
            d_p,
            f_name,
            d_name,
        })
    }

    fn dir_path(&mut self, dir: &str) -> Result<Vec<String>, anyhow::Error> {
        let mut output = vec![self.d_name.get(dir).context("")?.clone()];
        let mut cur = dir.to_owned();
        while let Some(parent) = self.d_p.get(&cur) {
            cur = parent.clone();
            output.push(
                self.d_name
                    .get(&cur)
                    .map(|x| x.to_owned())
                    .unwrap_or_else(|| {
                        println!("hey got to {}", &cur);
                        "".to_string()
                    })
                    .clone(),
            );
        }
        let _ = output.pop();
        output.reverse();
        Ok(output)
    }
    fn file_full_path(&mut self, file: &str) -> Result<Vec<String>, anyhow::Error> {
        let file_name = self
            .f_name
            .get(file)
            .map(|s| match s.split_once('|') {
                Some((_a, b)) => b,
                None => s,
            })
            .context("context")?
            .to_owned();

        let dir = self
            .f_c
            .get(file)
            .and_then(|c| self.c_d.get(c))
            .context("context")?
            .clone();
        let mut output = self.dir_path(&dir)?;
        output.push(file_name);
        Ok(output)
    }
}

fn main() -> Result<(), anyhow::Error> {
    let path = "sample/lib.msi";

    let mut msi = msi::open(path)?;
    let mut t = MsiTrav::new(&mut msi)?;
    let cab_name = msi.streams().find(|s| s.ends_with(".cab")).unwrap();
    let stream = msi.read_stream(&cab_name)?;
    let cab = Cabinet::new(stream)?;
    for ele in cab.folder_entries() {
        for ele in ele.file_entries() {
            println!("file in cab {}", ele.name());
            println!("deduced full path is {:?}", t.file_full_path(ele.name()));
        }
    }

    Ok(())
}
