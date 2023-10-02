use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs::OpenOptions,
    io::{BufReader, Error, Write},
    path::Path,
};

pub fn read_json<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> Result<T, Error> {
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

pub fn write_json<T: ?Sized + Serialize, P: AsRef<Path>>(
    value: &T,
    path: P,
) -> Result<usize, Error> {
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    let res = serde_json::to_string(value)?;
    f.write(res.as_bytes())
}
