use msi_extract::{Error, MsiExtractor};

fn main() -> Result<(), Error> {
    let path = "sample/lib.msi";
    let mut x = MsiExtractor::from_path(path)?;
    x.to("sample/out");

    Ok(())
}
