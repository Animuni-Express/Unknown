//! Browser-local UTF-8 filesystem. No operation reaches the host machine.
use std::{
    cell::RefCell,
    collections::BTreeMap,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    rc::Rc,
};

const MAX_BYTES: usize = 1_048_576;
const MAX_ENTRIES: usize = 256;
#[derive(Default)]
struct State {
    entries: BTreeMap<PathBuf, Option<Rc<RefCell<Vec<u8>>>>>,
    cwd: PathBuf,
}
thread_local! { static STATE: RefCell<State> = RefCell::new(State { entries: BTreeMap::from([(PathBuf::from("/"), None)]), cwd: PathBuf::from("/") }); }
fn err(kind: io::ErrorKind, text: &str) -> io::Error {
    io::Error::new(kind, text)
}
pub fn absolute(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let text = path
        .as_ref()
        .to_str()
        .ok_or_else(|| err(io::ErrorKind::InvalidInput, "path must be UTF-8"))?;
    if text.is_empty() || text.len() > 512 || text.contains(['\0', ':']) {
        return Err(err(io::ErrorKind::InvalidInput, "invalid virtual path"));
    }
    let text = text.replace('\\', "/");
    let mut result = if text.starts_with('/') {
        PathBuf::from("/")
    } else {
        current_dir()?
    };
    for part in text.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if !result.pop() {
                    return Err(err(
                        io::ErrorKind::PermissionDenied,
                        "path escapes virtual root",
                    ));
                }
            }
            p => result.push(p),
        }
    }
    Ok(result)
}
pub fn current_dir() -> io::Result<PathBuf> {
    Ok(STATE.with(|s| s.borrow().cwd.clone()))
}
pub fn set_current_dir(path: impl AsRef<Path>) -> io::Result<()> {
    let p = canonicalize(path)?;
    if !metadata(&p)?.is_dir() {
        return Err(err(io::ErrorKind::NotADirectory, "not a directory"));
    }
    STATE.with(|s| s.borrow_mut().cwd = p);
    Ok(())
}
pub fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let p = absolute(path)?;
    metadata(&p)?;
    Ok(p)
}
pub fn exists(path: impl AsRef<Path>) -> io::Result<bool> {
    let p = absolute(path)?;
    Ok(STATE.with(|s| s.borrow().entries.contains_key(&p)))
}
pub fn is_dir(path: impl AsRef<Path>) -> bool {
    metadata(path).is_ok_and(|m| m.is_dir())
}
#[derive(Clone, Copy)]
pub struct Metadata {
    dir: bool,
}
impl Metadata {
    pub fn is_dir(&self) -> bool {
        self.dir
    }
    pub fn is_file(&self) -> bool {
        !self.dir
    }
}
pub fn metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    let p = absolute(path)?;
    STATE.with(|s| {
        s.borrow()
            .entries
            .get(&p)
            .map(|v| Metadata { dir: v.is_none() })
            .ok_or_else(|| err(io::ErrorKind::NotFound, "path not found"))
    })
}
pub use metadata as symlink_metadata;
fn insert(path: PathBuf, data: Option<Rc<RefCell<Vec<u8>>>>) -> io::Result<()> {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.entries.len() >= MAX_ENTRIES {
            return Err(err(
                io::ErrorKind::StorageFull,
                "virtual filesystem entry limit reached",
            ));
        }
        if !s
            .entries
            .get(path.parent().unwrap_or(Path::new("/")))
            .is_some_and(Option::is_none)
        {
            return Err(err(
                io::ErrorKind::NotFound,
                "parent directory does not exist",
            ));
        }
        if s.entries.contains_key(&path) {
            return Err(err(io::ErrorKind::AlreadyExists, "destination exists"));
        }
        s.entries.insert(path, data);
        Ok(())
    })
}
pub fn create_dir(path: impl AsRef<Path>) -> io::Result<()> {
    insert(absolute(path)?, None)
}
pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    let mut result = String::new();
    File::open(path)?.read_to_string(&mut result)?;
    Ok(result)
}
pub fn write(path: impl AsRef<Path>, data: impl AsRef<[u8]>) -> io::Result<()> {
    let path = absolute(path)?;
    let bytes = data.as_ref();
    std::str::from_utf8(bytes).map_err(|_| {
        err(
            io::ErrorKind::InvalidData,
            "virtual files must remain UTF-8",
        )
    })?;
    let fits = STATE.with(|s| {
        let s = s.borrow();
        let total: usize = s
            .entries
            .iter()
            .filter(|(p, _)| **p != path)
            .filter_map(|(_, v)| v.as_ref())
            .map(|v| v.borrow().len())
            .sum();
        total.saturating_add(bytes.len()) <= MAX_BYTES
    });
    if !fits {
        return Err(err(
            io::ErrorKind::StorageFull,
            "virtual filesystem exceeds 1 MiB",
        ));
    }
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?
        .write_all(bytes)
}
pub fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    let p = absolute(path)?;
    if metadata(&p)?.is_dir() {
        return Err(err(
            io::ErrorKind::IsADirectory,
            "cannot delete directory as file",
        ));
    }
    STATE.with(|s| s.borrow_mut().entries.remove(&p));
    Ok(())
}
pub fn remove_dir(path: impl AsRef<Path>) -> io::Result<()> {
    let p = absolute(path)?;
    if !metadata(&p)?.is_dir() {
        return Err(err(io::ErrorKind::NotADirectory, "not a directory"));
    }
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if p == Path::new("/") || s.cwd.starts_with(&p) {
            return Err(err(
                io::ErrorKind::PermissionDenied,
                "cannot delete root or current directory",
            ));
        }
        if s.entries.keys().any(|k| k != &p && k.starts_with(&p)) {
            return Err(err(
                io::ErrorKind::DirectoryNotEmpty,
                "directory is not empty",
            ));
        }
        s.entries.remove(&p);
        Ok(())
    })
}
pub fn rename_noreplace(src: &Path, dst: &Path) -> io::Result<()> {
    let src = canonicalize(src)?;
    let dst = absolute(dst)?;
    if src == Path::new("/") || dst.starts_with(&src) {
        return Err(err(io::ErrorKind::InvalidInput, "invalid move"));
    }
    if exists(&dst)? {
        return Err(err(io::ErrorKind::AlreadyExists, "destination exists"));
    }
    if !is_dir(dst.parent().unwrap_or(Path::new("/"))) {
        return Err(err(io::ErrorKind::NotFound, "destination parent missing"));
    }
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let keys: Vec<_> = s
            .entries
            .keys()
            .filter(|p| p.starts_with(&src))
            .cloned()
            .collect();
        for old in keys {
            let new = dst.join(old.strip_prefix(&src).unwrap());
            let v = s.entries.remove(&old).unwrap();
            s.entries.insert(new, v);
        }
        if s.cwd.starts_with(&src) {
            s.cwd = dst.join(s.cwd.strip_prefix(&src).unwrap());
        }
    });
    Ok(())
}
pub struct DirEntry {
    path: PathBuf,
    dir: bool,
}
impl DirEntry {
    pub fn file_name(&self) -> std::ffi::OsString {
        self.path.file_name().unwrap().into()
    }
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }
    pub fn file_type(&self) -> io::Result<Metadata> {
        Ok(Metadata { dir: self.dir })
    }
}
pub fn read_dir(path: impl AsRef<Path>) -> io::Result<std::vec::IntoIter<io::Result<DirEntry>>> {
    let p = canonicalize(path)?;
    if !is_dir(&p) {
        return Err(err(io::ErrorKind::NotADirectory, "not a directory"));
    }
    Ok(STATE.with(|s| {
        s.borrow()
            .entries
            .iter()
            .filter(|(k, _)| k.as_path() != p && k.parent() == Some(p.as_path()))
            .map(|(k, v)| {
                Ok(DirEntry {
                    path: k.clone(),
                    dir: v.is_none(),
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
    }))
}
#[derive(Debug)]
pub struct File {
    data: Rc<RefCell<Vec<u8>>>,
    cursor: usize,
    readable: bool,
    writable: bool,
    append: bool,
}
impl File {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        OpenOptions::new().read(true).open(path)
    }
    pub fn metadata(&self) -> io::Result<Metadata> {
        Ok(Metadata { dir: false })
    }
}
impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.readable {
            return Err(err(io::ErrorKind::PermissionDenied, "file is not readable"));
        }
        let data = self.data.borrow();
        if self.cursor >= data.len() {
            return Ok(0);
        }
        let count = buf.len().min(data.len().saturating_sub(self.cursor));
        buf[..count].copy_from_slice(&data[self.cursor..self.cursor + count]);
        self.cursor += count;
        Ok(count)
    }
}
impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !self.writable {
            return Err(err(io::ErrorKind::PermissionDenied, "file is not writable"));
        }
        if !STATE.with(|s| {
            s.borrow()
                .entries
                .values()
                .flatten()
                .any(|v| Rc::ptr_eq(v, &self.data))
        }) {
            return Err(err(io::ErrorKind::NotFound, "file was deleted"));
        }
        let total: usize = STATE.with(|s| {
            s.borrow()
                .entries
                .values()
                .flatten()
                .map(|v| v.borrow().len())
                .sum()
        });
        let mut data = self.data.borrow_mut();
        if self.append {
            self.cursor = data.len();
        }
        let end = self
            .cursor
            .checked_add(buf.len())
            .ok_or_else(|| err(io::ErrorKind::StorageFull, "file too large"))?;
        if end > MAX_BYTES || total.saturating_add(end.saturating_sub(data.len())) > MAX_BYTES {
            return Err(err(
                io::ErrorKind::StorageFull,
                "virtual filesystem exceeds 1 MiB",
            ));
        }
        let mut updated = data.clone();
        if end > updated.len() {
            updated.resize(end, 0);
        }
        updated[self.cursor..end].copy_from_slice(buf);
        std::str::from_utf8(&updated).map_err(|_| {
            err(
                io::ErrorKind::InvalidData,
                "virtual files must remain UTF-8",
            )
        })?;
        *data = updated;
        self.cursor = end;
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[derive(Default)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    create: bool,
    create_new: bool,
    truncate: bool,
}
impl OpenOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn read(&mut self, v: bool) -> &mut Self {
        self.read = v;
        self
    }
    pub fn write(&mut self, v: bool) -> &mut Self {
        self.write = v;
        self
    }
    pub fn append(&mut self, v: bool) -> &mut Self {
        self.append = v;
        self
    }
    pub fn create(&mut self, v: bool) -> &mut Self {
        self.create = v;
        self
    }
    pub fn create_new(&mut self, v: bool) -> &mut Self {
        self.create_new = v;
        self
    }
    pub fn truncate(&mut self, v: bool) -> &mut Self {
        self.truncate = v;
        self
    }
    pub fn open(&self, path: impl AsRef<Path>) -> io::Result<File> {
        let p = absolute(path)?;
        if self.create_new && exists(&p)? {
            return Err(err(io::ErrorKind::AlreadyExists, "destination exists"));
        }
        if !exists(&p)? && (self.create || self.create_new) {
            insert(p.clone(), Some(Rc::new(RefCell::new(Vec::new()))))?;
        }
        let data = STATE
            .with(|s| s.borrow().entries.get(&p).cloned())
            .ok_or_else(|| err(io::ErrorKind::NotFound, "file not found"))?
            .ok_or_else(|| err(io::ErrorKind::IsADirectory, "not a file"))?;
        if self.truncate && self.write {
            data.borrow_mut().clear();
        }
        Ok(File {
            data,
            cursor: 0,
            readable: self.read,
            writable: self.write || self.append,
            append: self.append,
        })
    }
}
pub fn snapshot() -> Vec<(String, Option<String>)> {
    STATE.with(|s| {
        s.borrow()
            .entries
            .iter()
            .map(|(p, v)| {
                (
                    p.to_string_lossy().into_owned(),
                    v.as_ref()
                        .map(|v| String::from_utf8_lossy(&v.borrow()).into_owned()),
                )
            })
            .collect()
    })
}
pub fn reset() {
    STATE.with(|s| {
        *s.borrow_mut() = State {
            entries: BTreeMap::from([(PathBuf::from("/"), None)]),
            cwd: PathBuf::from("/"),
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn files_are_shared_bounded_and_confined() {
        reset();
        create_dir("/work").unwrap();
        set_current_dir("/work").unwrap();
        write("a.unk", "hello").unwrap();
        let mut handle = OpenOptions::new().append(true).open("a.unk").unwrap();
        handle.write_all(b"!").unwrap();
        assert_eq!(read_to_string("./a.unk").unwrap(), "hello!");
        assert!(absolute("../../escape").is_err());
        assert!(write("large", vec![0; MAX_BYTES + 1]).is_err());
        assert!(rename_noreplace(Path::new("a.unk"), Path::new("a.unk")).is_err());
        rename_noreplace(Path::new("a.unk"), Path::new("b.unk")).unwrap();
        assert_eq!(read_to_string("b.unk").unwrap(), "hello!");
        let mut reader = File::open("b.unk").unwrap();
        let mut text = String::new();
        reader.read_to_string(&mut text).unwrap();
        write("b.unk", "").unwrap();
        assert_eq!(reader.read(&mut [0; 1]).unwrap(), 0);
        remove_file("b.unk").unwrap();
        assert!(handle.write_all(b"detached").is_err());
        assert!(remove_dir("/work").is_err());
        reset();
    }
}
