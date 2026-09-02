use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::OpenOptions,
    io::{BufReader, Error, Write},
    path::Path,
};

pub fn read_json<T, P>(path: P) -> Result<T, Error>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    match OpenOptions::new().read(true).open(path) {
        Ok(f) => {
            let reader = BufReader::new(f);
            match serde_json::from_reader(reader) {
                Ok(t) => Ok(t),
                Err(err) => Err(err.into()),
            }
        }
        Err(err) => Err(err),
    }
}

pub fn write_json<T, P>(value: &T, path: P) -> Result<usize, Error>
where
    T: ?Sized + Serialize,
    P: AsRef<Path>,
{
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    let res = serde_json::to_string(value)?;
    // `write_all`, not `write`: a short write would leave truncated JSON on disk
    // and still report success (with a smaller byte count) to the caller.
    let bytes = res.len();
    f.write_all(res.as_bytes())?;
    Ok(bytes)
}
