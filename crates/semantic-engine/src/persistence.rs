use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

/// Load a serializable struct from a bincode file
pub fn load_bincode<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let data = bincode::deserialize_from(reader)?;
    Ok(data)
}

/// Save a serializable struct to a bincode file (atomically via rename)
pub fn save_bincode<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    // Write to a temporary file first
    let tmp_path = path.with_extension("tmp");
    {
        let file = File::create(&tmp_path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, data)?;
    }
    // Rename to target path (atomic)
    std::fs::rename(tmp_path, path)?;
    Ok(())
}
