use msi_extract::{Error, MsiExtractor};

fn main() -> Result<(), Error> {
    let path = "sample/lib.msi";

    // let file = File::open(path)?;
    let mut x = MsiExtractor::from_path(path)?;
    // let mut x = MsiExtractor::from_reader(file)?;

    x.to("sample/out");

    Ok(())
}