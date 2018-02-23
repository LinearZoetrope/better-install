use std::path::{Path, PathBuf};
use error;

mod name_path;
mod cd_manager;

pub use self::name_path::NameOrPath;
pub use self::cd_manager::CdManager;

/// Fetches a given file from the URL into a byte buffer.
///
/// If no buffer is provided, an empty one will be allocated for you.
/// The buffer used will always be returned if the function is successful.
///
/// This is useful for sharing big pre-allocated buffers between calls.
pub fn curl(url: &str, buf: Option<Vec<u8>>) -> error::Result<Vec<u8>> {
    use curl::easy::{Easy2, Handler, WriteError};

    let mut buf = buf.unwrap_or_default();

    {
        struct Collector<'a>(&'a mut Vec<u8>);
        impl<'a> Handler for Collector<'a> {
            fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
                self.0.extend_from_slice(data);
                Ok(data.len())
            }
        }

        let mut curl = Easy2::new(Collector(&mut buf));
        curl.follow_location(true)?;
        curl.url(url)?;
        curl.perform()?;
    }

    Ok(buf)
}

/// Unzips the given byte buffer into the path indicated by `path_root`.
///
/// The `into` parameter indicates whether or not the zip should be extracted "into" the current
/// directory or not. For instance, most zip files have a top-level folder named the same as the zip,
/// so "foo.zip" extracts to the folder "./foo". The into parameter overrides this and essentially
/// "foo/*" directly into ".". You could consider it shorthand for `unzip foo.zip`
/// followed by `mv foo/* .` and `rm foo`.
// Modified from the `zip` github Repo, see ATTRIBUTIONS in the crate root for more info
pub fn unzip(buf: &[u8], mut path_root: CdManager, into: bool) -> error::Result<()> {
    use std::io::Cursor;
    use std::io;
    use std::fs;
    use zip::ZipArchive;

    let mut archive = ZipArchive::new(Cursor::new(buf))?;

    let parent_name = if into {
        sanitize_filename(archive.by_index(0)?.name())
    } else {
        Path::new("").to_path_buf()
    };

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let mut outpath = sanitize_filename(file.name());
        let outpath = if into {
            outpath.strip_prefix(&parent_name)?
        } else {
            &outpath
        };

        let mut path_root = path_root.layer();
        path_root.push(&outpath);

        let outpath = path_root.as_ref();

        if (&*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }

    Ok(())
}

// Taken from the `zip` github Repo, see ATTRIBUTIONS in the crate root for more info
fn sanitize_filename(filename: &str) -> PathBuf {
    use std::path::Component;

    let no_null_filename = match filename.find('\0') {
        Some(index) => &filename[0..index],
        None => filename,
    };

    Path::new(no_null_filename)
        .components()
        .filter(|component| match *component {
            Component::Normal(..) => true,
            _ => false,
        })
        .fold(PathBuf::new(), |mut path, ref cur| {
            path.push(cur.as_os_str());
            path
        })
}

#[cfg(windows)]
pub fn make_deletable<P: AsRef<Path>>(target: P) -> error::Result<()> {
    use walkdir::WalkDir;
    use std::fs;

    let wd = WalkDir::new(target);
    for entry in wd {
        let entry = entry?;
        let metadata = entry.metadata()?;

        // Folders are always readonly in windows
        if metadata.is_file() {
            let mut perm = metadata.permissions();
            perm.set_readonly(false);
            fs::set_permissions(entry.path(), perm)?;
        }
    }

    Ok(())
}
