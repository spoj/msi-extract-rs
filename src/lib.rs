use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek},
    path::{Path, PathBuf},
};

use cab::Cabinet;
use msi::{Package, Select, StreamReader};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Contract(#[from] std::io::Error),
    #[error("Interal error")]
    Other,
}
struct MsiTraverser {
    f_c: HashMap<String, String>,
    c_d: HashMap<String, String>,
    d_p: HashMap<String, String>,
    f_name: HashMap<String, String>,
    d_name: HashMap<String, String>,
}

impl MsiTraverser {
    fn new<F>(package: &mut Package<F>) -> Result<Self, Error>
    where
        F: Read + Seek,
    {
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

    fn dir_path(&mut self, dir: &str) -> Result<Vec<String>, Error> {
        let mut output = vec![self.d_name.get(dir).ok_or(Error::Other)?.clone()];
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
    fn resolve(&mut self, file: &str) -> Result<PathBuf, Error> {
        let file_name = self
            .f_name
            .get(file)
            .map(|s| match s.split_once('|') {
                Some((_a, b)) => b,
                None => s,
            })
            .ok_or(Error::Other)?
            .to_owned();

        let dir = self
            .f_c
            .get(file)
            .and_then(|c| self.c_d.get(c))
            .ok_or(Error::Other)?
            .clone();
        let mut output = PathBuf::new();
        output.extend(self.dir_path(&dir)?);
        output.push(file_name);
        Ok(output)
    }
}

pub struct MsiExtractor<F> {
    _package: msi::Package<F>,
    cab: Cabinet<StreamReader<F>>,
    traverser: MsiTraverser,
}

impl<F> MsiExtractor<F>
where
    F: Read + Seek,
{
    pub fn from_msi(mut package: Package<F>) -> Result<Self, Error> {
        let traverser = MsiTraverser::new(&mut package)?;
        let cab_name = package
            .streams()
            .find(|s| s.ends_with(".cab"))
            .ok_or(Error::Other)?;
        let stream = package.read_stream(&cab_name)?;
        let cab = Cabinet::new(stream)?;

        Ok(MsiExtractor {
            _package: package,
            cab,
            traverser,
        })
    }
    pub fn from_reader(reader: F) -> Result<MsiExtractor<F>, Error> {
        let mut package = msi::Package::open(reader)?;
        let traverser = MsiTraverser::new(&mut package)?;
        let cab_name = package
            .streams()
            .find(|s| s.ends_with(".cab"))
            .ok_or(Error::Other)?;
        let stream = package.read_stream(&cab_name)?;
        let cab = Cabinet::new(stream)?;

        Ok(MsiExtractor {
            _package: package,
            cab,
            traverser,
        })
    }
    pub fn to<P: AsRef<Path>>(&mut self, target: P) {
        self.cab
            .folder_entries()
            .flat_map(|f| f.file_entries())
            .for_each(|f| {
                let file = f.name();
                if let Ok(path) = self.traverser.resolve(file) {
                    let target = target.as_ref().join(path);
                    if let Some(dir) = target.parent()
                        && let Ok(()) = std::fs::create_dir_all(dir)
                        && let Ok(mut target_file) = File::create(&target)
                    {
                        let mut data = file.as_bytes();
                        let _ = std::io::copy(&mut data, &mut target_file);
                        // println!("writing {} bytes to {:?}", data.len(), &target);
                    }
                }
            });
    }
}

impl MsiExtractor<File> {
    pub fn from_path<P>(path: P) -> Result<MsiExtractor<File>, Error>
    where
        P: AsRef<Path>,
    {
        let reader = File::open(path)?;
        let mut package = msi::Package::open(reader)?;
        let traverser = MsiTraverser::new(&mut package)?;
        let cab_name = package
            .streams()
            .find(|s| s.ends_with(".cab"))
            .ok_or(Error::Other)?;
        let stream = package.read_stream(&cab_name)?;
        let cab = Cabinet::new(stream)?;

        Ok(MsiExtractor {
            _package: package,
            cab,
            traverser,
        })
    }
}
