#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::ffi::CString;
use std::ffi::{c_char, CStr};
pub mod vfs;
#[cfg(target_arch = "wasm32")]
pub use vfs as filesystem;
#[cfg(not(target_arch = "wasm32"))]
pub mod filesystem {
    pub use std::env::{current_dir, set_current_dir};
    pub use std::fs::*;
    pub use std::path::absolute;
    pub fn exists(path: impl AsRef<std::path::Path>) -> std::io::Result<bool> {
        path.as_ref().try_exists()
    }
    pub fn is_dir(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().is_dir()
    }
}
use filesystem::{self as fs, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;

const EXIT: i32 = -1;
const CONFIRM: i32 = -2;

fn usage(command: &str) -> &'static str {
    match command {
        "list" => "list [path] [--all|--hidden] [--sort=name] — list directory contents",
        "goto" => "goto [path] — change directory (default: home)",
        "back" => "back — go to the parent directory",
        "where" => "where — show the current directory",
        "make" => "make file|folder <path> — create a file or directory",
        "read" => "read <file> — display a file",
        "copy" => "copy <source> <destination> — copy a file without overwriting",
        "move" => "move <source> <destination> — move without overwriting",
        "delete" => "delete <path> [--force] — delete a file or empty directory",
        "clear" => "clear — clear the screen",
        "cls" => "cls — clear the screen (alias for clear)",
        "help" => "help — show available commands",
        "exit" => "exit — leave USH",
        _ => "",
    }
}

fn arguments(args: &[String], min: usize, max: usize) -> Result<&[String], String> {
    let rest = &args[1..];
    if rest.iter().any(|v| v.starts_with('-')) {
        return Err("invalid option".into());
    }
    if rest.len() < min || rest.len() > max {
        return Err(format!("usage: {}", usage(&args[0])));
    }
    Ok(rest)
}

fn error(path: &Path, err: io::Error) -> String {
    if err.kind() == io::ErrorKind::NotFound {
        format!("path not found: {}", path.display())
    } else if err.kind() == io::ErrorKind::AlreadyExists {
        format!("destination already exists: {}", path.display())
    } else {
        format!("{}: {}", path.display(), err)
    }
}

fn destination(source: &Path, target: &Path) -> Result<PathBuf, String> {
    if fs::is_dir(target) {
        let name = source
            .file_name()
            .ok_or_else(|| "source has no file name".to_string())?;
        Ok(target.join(name))
    } else {
        Ok(target.to_path_buf())
    }
}

#[cfg(target_os = "linux")]
fn rename_noreplace(src: &Path, dst: &Path) -> io::Result<()> {
    use std::os::raw::{c_char, c_int, c_uint};
    use std::os::unix::ffi::OsStrExt;
    unsafe extern "C" {
        fn renameat2(
            olddir: c_int,
            old: *const c_char,
            newdir: c_int,
            new: *const c_char,
            flags: c_uint,
        ) -> c_int;
    }
    let src = CString::new(src.as_os_str().as_bytes()).map_err(|_| io::ErrorKind::InvalidInput)?;
    let dst = CString::new(dst.as_os_str().as_bytes()).map_err(|_| io::ErrorKind::InvalidInput)?;
    if unsafe { renameat2(-100, src.as_ptr(), -100, dst.as_ptr(), 1) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "macos")]
fn rename_noreplace(src: &Path, dst: &Path) -> io::Result<()> {
    use std::os::raw::{c_char, c_int, c_uint};
    use std::os::unix::ffi::OsStrExt;
    unsafe extern "C" {
        fn renamex_np(old: *const c_char, new: *const c_char, flags: c_uint) -> c_int;
    }
    let src = CString::new(src.as_os_str().as_bytes()).map_err(|_| io::ErrorKind::InvalidInput)?;
    let dst = CString::new(dst.as_os_str().as_bytes()).map_err(|_| io::ErrorKind::InvalidInput)?;
    if unsafe { renamex_np(src.as_ptr(), dst.as_ptr(), 4) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn rename_noreplace(src: &Path, dst: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileW(old: *const u16, new: *const u16) -> i32;
    }
    let src: Vec<u16> = src.as_os_str().encode_wide().chain(Some(0)).collect();
    let dst: Vec<u16> = dst.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe { MoveFileW(src.as_ptr(), dst.as_ptr()) } != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(any(target_os = "linux", target_os = "macos", windows))
))]
fn rename_noreplace(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "safe move is not supported on this platform",
    ))
}

#[cfg(target_arch = "wasm32")]
fn rename_noreplace(src: &Path, dst: &Path) -> io::Result<()> {
    fs::rename_noreplace(src, dst)
}

fn execute(args: &[String]) -> Result<i32, String> {
    execute_to(args, &mut io::stdout().lock())
}

/// Run the same USH commands with explicit output capture.
pub fn execute_to(args: &[String], output: &mut dyn Write) -> Result<i32, String> {
    if args.is_empty() {
        return Ok(0);
    }
    let command = args[0].as_str();
    let builtins = [
        "list", "goto", "back", "where", "make", "read", "copy", "move", "delete", "clear", "cls",
        "help", "exit",
    ];
    if builtins.contains(&command) && args.len() == 2 && args[1] == "--help" {
        writeln!(output, "{}", usage(command)).map_err(|e| e.to_string())?;
        return Ok(0);
    }
    match command {
        "list" => {
            let mut path = None;
            let mut hidden = false;
            let mut sort = false;
            for arg in &args[1..] {
                match arg.as_str() {
                    "--all" | "--hidden" => hidden = true,
                    "--sort=name" => sort = true,
                    s if s.starts_with('-') => return Err(format!("invalid option: {s}")),
                    s if path.is_none() => path = Some(s),
                    _ => return Err(format!("usage: {}", usage(command))),
                }
            }
            let path = Path::new(path.unwrap_or("."));
            let entries = fs::read_dir(path).map_err(|e| error(path, e))?;
            let mut entries = entries
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| error(path, e))?;
            if sort {
                entries.sort_by_key(|e| e.file_name());
            }
            for entry in entries {
                let name = entry.file_name();
                if !hidden && is_hidden(&entry, &name)? {
                    continue;
                }
                let ty = entry.file_type().map_err(|e| error(&entry.path(), e))?;
                writeln!(
                    output,
                    "{}  {}",
                    if ty.is_dir() { "folder" } else { "file  " },
                    name.to_string_lossy()
                )
                .map_err(|e| e.to_string())?;
            }
        }
        "goto" | "back" => {
            let a = arguments(args, 0, if command == "back" { 0 } else { 1 })?;
            let home;
            let path = if command == "back" {
                Path::new("..")
            } else if a.is_empty() {
                home = home_dir().ok_or("home directory is unavailable")?;
                home.as_path()
            } else {
                Path::new(&a[0])
            };
            fs::set_current_dir(path).map_err(|e| error(path, e))?;
        }
        "where" => {
            arguments(args, 0, 0)?;
            writeln!(
                output,
                "{}",
                fs::current_dir().map_err(|e| e.to_string())?.display()
            )
            .map_err(|e| e.to_string())?;
        }
        "make" => {
            let a = arguments(args, 2, 2)?;
            let path = Path::new(&a[1]);
            match a[0].as_str() {
                "file" => {
                    OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(path)
                        .map_err(|e| error(path, e))?;
                }
                "folder" => fs::create_dir(path).map_err(|e| error(path, e))?,
                _ => return Err(format!("usage: {}", usage(command))),
            }
        }
        "read" => {
            let a = arguments(args, 1, 1)?;
            let path = Path::new(&a[0]);
            let mut file = fs::File::open(path).map_err(|e| error(path, e))?;
            io::copy(&mut file, output).map_err(|e| error(path, e))?;
        }
        "copy" => {
            let a = arguments(args, 2, 2)?;
            let source = Path::new(&a[0]);
            let target = destination(source, Path::new(&a[1]))?;
            let mut input = fs::File::open(source).map_err(|e| error(source, e))?;
            if !input.metadata().map_err(|e| error(source, e))?.is_file() {
                return Err(format!("not a file: {}", source.display()));
            }
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .map_err(|e| error(&target, e))?;
            #[cfg(target_arch = "wasm32")]
            let copied = std::io::read_to_string(&mut input)
                .and_then(|text| output.write_all(text.as_bytes()));
            #[cfg(not(target_arch = "wasm32"))]
            let copied = io::copy(&mut input, &mut output).map(|_| ());
            if let Err(e) = copied {
                drop(output);
                let _ = fs::remove_file(&target);
                return Err(error(&target, e));
            }
        }
        "move" => {
            let a = arguments(args, 2, 2)?;
            let source = Path::new(&a[0]);
            let target = destination(source, Path::new(&a[1]))?;
            rename_noreplace(source, &target).map_err(|e| error(&target, e))?;
        }
        "delete" => {
            let mut path = None;
            let mut force = false;
            for arg in &args[1..] {
                if arg == "--force" && !force {
                    force = true;
                } else if arg.starts_with('-') {
                    return Err(format!("invalid option: {arg}"));
                } else if path.is_none() {
                    path = Some(Path::new(arg));
                } else {
                    return Err(format!("usage: {}", usage(command)));
                }
            }
            let path = path.ok_or_else(|| format!("usage: {}", usage(command)))?;
            let metadata = fs::symlink_metadata(path).map_err(|e| error(path, e))?;
            if !force {
                return Ok(CONFIRM);
            }
            if is_directory_entry(&metadata) {
                fs::remove_dir(path)
            } else {
                fs::remove_file(path)
            }
            .map_err(|e| error(path, e))?;
        }
        "clear" | "cls" => {
            arguments(args, 0, 0)?;
            clear_screen_to(output).map_err(|e| e.to_string())?;
        }
        "help" => {
            arguments(args, 0, 0)?;
            for command in builtins {
                writeln!(output, "{}", usage(command)).map_err(|e| e.to_string())?;
            }
        }
        "exit" => {
            arguments(args, 0, 0)?;
            return Ok(EXIT);
        }
        _ => {
            #[cfg(target_arch = "wasm32")]
            return Err(format!(
                "native process execution is unavailable in the browser: {command}"
            ));
            #[cfg(not(target_arch = "wasm32"))]
            {
                let status = Command::new(command)
                    .args(&args[1..])
                    .status()
                    .map_err(|e| {
                        if e.kind() == io::ErrorKind::NotFound {
                            format!("command not found: {command}")
                        } else {
                            format!("{command}: {e}")
                        }
                    })?;
                return Ok(status.code().unwrap_or(1).max(0));
            }
        }
    }
    Ok(0)
}

pub fn clear_screen_to(output: &mut dyn Write) -> io::Result<()> {
    #[cfg(windows)]
    let console = enable_virtual_terminal()?;
    let result = output
        .write_all(b"\x1b[2J\x1b[3J\x1b[H")
        .and_then(|()| output.flush());
    #[cfg(windows)]
    if let Some((handle, mode)) = console {
        unsafe { SetConsoleMode(handle, mode) };
    }
    result
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetStdHandle(which: u32) -> *mut std::ffi::c_void;
    fn GetConsoleMode(handle: *mut std::ffi::c_void, mode: *mut u32) -> i32;
    fn SetConsoleMode(handle: *mut std::ffi::c_void, mode: u32) -> i32;
}

#[cfg(windows)]
fn enable_virtual_terminal() -> io::Result<Option<(*mut std::ffi::c_void, u32)>> {
    use std::io::IsTerminal;
    if !io::stdout().is_terminal() {
        return Ok(None);
    }
    unsafe {
        let handle = GetStdHandle(-11i32 as u32);
        let mut mode = 0;
        if GetConsoleMode(handle, &mut mode) == 0 || SetConsoleMode(handle, mode | 0x0004) == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Some((handle, mode)))
    }
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(target_arch = "wasm32")]
    {
        return Some(PathBuf::from("/"));
    }
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(all(not(windows), not(target_arch = "wasm32")))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

fn is_directory_entry(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        metadata.is_dir() || metadata.file_type().is_symlink_dir()
    }
    #[cfg(not(windows))]
    {
        metadata.is_dir()
    }
}
fn is_hidden(entry: &fs::DirEntry, name: &std::ffi::OsStr) -> Result<bool, String> {
    let hidden = name.to_string_lossy().starts_with('.');
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        Ok(hidden
            || entry
                .metadata()
                .map_err(|e| error(&entry.path(), e))?
                .file_attributes()
                & 2
                != 0)
    }
    #[cfg(not(windows))]
    {
        let _ = entry;
        Ok(hidden)
    }
}

/// Executes UTF-8 arguments borrowed for this call; no pointers are retained.
/// Returns -1 for exit, -2 for confirmation, otherwise a process-style status.
///
/// # Safety
/// `argv` must point to `argc` valid, NUL-terminated UTF-8 strings for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn ush_execute(argc: i32, argv: *const *const c_char) -> i32 {
    let result = std::panic::catch_unwind(|| {
        if argc < 0 || (argc > 0 && argv.is_null()) {
            return Err("invalid command arguments".to_string());
        }
        let mut args = Vec::with_capacity(argc as usize);
        for i in 0..argc as usize {
            let ptr = *argv.add(i);
            if ptr.is_null() {
                return Err("invalid command argument".into());
            }
            let arg = CStr::from_ptr(ptr)
                .to_str()
                .map_err(|_| "command argument is not valid UTF-8")?;
            args.push(arg.to_owned());
        }
        execute(&args)
    });
    match result {
        Ok(Ok(code)) => code,
        Ok(Err(message)) => {
            eprintln!("ush: {message}");
            1
        }
        Err(_) => {
            eprintln!("ush: internal error");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn command(args: &[&str]) -> Result<i32, String> {
        execute(&args.iter().map(|v| v.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn files_are_safe_and_validated() {
        let root = std::env::temp_dir().join(format!(
            "ush-core-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let folder = root.join("folder");
        let file = folder.join("file.txt");
        let copied = folder.join("copy.txt");
        let moved = folder.join("moved.txt");
        let root_s = root.to_str().unwrap();
        let folder_s = folder.to_str().unwrap();
        let file_s = file.to_str().unwrap();
        let copy_s = copied.to_str().unwrap();
        let moved_s = moved.to_str().unwrap();
        assert_eq!(command(&["make", "folder", folder_s]), Ok(0));
        assert_eq!(command(&["make", "file", file_s]), Ok(0));
        fs::write(&file, b"contents").unwrap();
        assert!(command(&["make", "file", file_s]).is_err());
        assert_eq!(fs::read(&file).unwrap(), b"contents");
        assert_eq!(command(&["list", folder_s, "--all", "--sort=name"]), Ok(0));
        assert!(command(&["list", folder_s, "--bad"]).is_err());
        assert_eq!(command(&["read", file_s]), Ok(0));
        assert_eq!(command(&["copy", file_s, copy_s]), Ok(0));
        assert!(command(&["copy", file_s, copy_s]).is_err());
        assert_eq!(command(&["move", copy_s, moved_s]), Ok(0));
        assert!(command(&["move", moved_s, file_s]).is_err());
        assert_eq!(fs::read(&file).unwrap(), b"contents");
        assert_eq!(command(&["delete", moved_s]), Ok(CONFIRM));
        assert!(moved.exists());
        assert!(command(&["delete", folder_s, "--force"]).is_err());
        assert_eq!(command(&["delete", moved_s, "--force"]), Ok(0));
        assert_eq!(command(&["delete", file_s, "--force"]), Ok(0));
        assert_eq!(command(&["delete", folder_s, "--force"]), Ok(0));
        assert!(command(&["read", file_s]).is_err());
        assert!(command(&["make", "bad", file_s]).is_err());
        assert_eq!(command(&["goto", "--help"]), Ok(0));
        assert_eq!(command(&["exit"]), Ok(EXIT));
        assert!(command(&["ush_command_that_does_not_exist_93044"])
            .unwrap_err()
            .contains("command not found"));
        assert_eq!(command(&["rustc", "--version"]), Ok(0));
        fs::remove_dir(root_s).unwrap();
    }
}
