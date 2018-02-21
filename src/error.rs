error_chain! {
    foreign_links {
        Fmt(::std::fmt::Error);
        Io(::std::io::Error);
        Curl(::curl::Error);
        Zip(::zip::result::ZipError);
        StripPrefix(::std::path::StripPrefixError);
    Git(::git2::Error) #[cfg(unix)];
    WalkDir(::walkdir::Error) #[cfg(windows)];
    }

    errors {
        CannotCleanError(path: String) {
            description("cannot clean target directory")
            display("cannot clean target directory: '{}'", path)
        }

        CannotCreateError(path: String) {
            description("cannot create target path")
            display("cannot create target path: '{}'", path)
        }

        GetFailure {
            description("could not execute get subcommand")
            display("could not execute get subcommand")
        }

        MultiError(errors: MultiError) {
            description("multiple errors ocurred in parallel")
            display("multiple errors ocurred in parallel: {}", errors)
        }
    }
}

pub const CLEAN_EXIT: i32 = 0;

#[derive(Debug)]
pub struct MultiError {
    pub errors: Vec<Error>,
}

impl ::std::fmt::Display for MultiError {
    fn fmt(
        &self,
        formatter: &mut ::std::fmt::Formatter,
    ) -> ::std::result::Result<(), ::std::fmt::Error> {
        for (i, error) in self.errors.iter().enumerate() {
            write!(formatter, "Error {}: {}\n", i, error)?;
        }

        Ok(())
    }
}
