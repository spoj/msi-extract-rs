use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

use object::Object;
use sevenz_rust::{Password, SevenZReader};
fn get_nsis_data_start_offset(data: &[u8]) -> io::Result<usize> {
    let obj_file = object::File::parse(data).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse file: {}", e),
        )
    })?;

    let pe_file = match obj_file {
        object::File::Pe32(pe_file) => pe_file,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not a PE (Windows Executable) file",
            ));
        }
    };

    let mut max_end_offset: usize = 0;

    // Find the end of image data, usually indicated by pointer to raw data + size of raw data of the last section.
    // We iterate through all sections to find the maximum possible end.
    for (_, section) in pe_file.sections(). {
        // Ensure the section points to valid data within the file bounds
        if section.pointer_to_raw_data() > data.len() as u32 {
            continue; // Skip invalid sections
        }

        let section_end_in_file =
            section.pointer_to_raw_data() as usize + section.size_of_raw_data() as usize;

        if section_end_in_file > max_end_offset {
            max_end_offset = section_end_in_file;
        }
    }

    if max_end_offset == 0 {
        // This could happen for very small, odd PE files, or if parsing failed critically.
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Could not determine end of PE file data.",
        ));
    }

    Ok(max_end_offset)
}

fn extract_paf(archive_path: &Path, output_dir: &Path) -> Result<(), anyhow::Error> {
    const SEVEN_ZIP_MAGIC_BYTES: [u8; 6] = [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C];

    println!("Reading file: {}", archive_path.display());
    let mut file = File::open(archive_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    println!("Searching for 7-Zip magic bytes...");
    let sevenz_start_idx =
        memchr::memmem::find(&data, &SEVEN_ZIP_MAGIC_BYTES).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "7-Zip signature not found in paf.exe",
            )
        })?;

    println!("Found 7-Zip archive starting at offset: {sevenz_start_idx}");
    let sevenz_data = &data[sevenz_start_idx..];

    // Create a cursor for the in-memory 7-Zip data
    let mut reader_stream = io::Cursor::new(sevenz_data);

    // Initialize the 7-Zip reader
    let mut sevenz_archive = SevenZReader::new(
        &mut reader_stream,
        sevenz_data.len() as u64,
        Password::empty(),
    )?;

    // Prepare output directory
    std::fs::create_dir_all(output_dir)?;
    println!("Extracting to: {}", output_dir.display());

    // Extract all entries
    sevenz_archive.for_each_entries(|entry, mut r| {
        let entry_path = output_dir.join(&entry.name);

        if entry.is_directory() {
            std::fs::create_dir_all(&entry_path)?;
        } else {
            if let Some(parent) = entry_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut output_file = File::create(&entry_path)?;

            io::copy(&mut r, &mut output_file)?;
        }
        println!("  Extracted: {}", entry_path.display());

        Ok(true)
    })?;

    Ok(())
}
