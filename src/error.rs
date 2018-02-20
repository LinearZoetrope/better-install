error_chain! {
    foreign_links {
        Fmt(::std::fmt::Error);
        Io(::std::io::Error);
	Git(::git2::Error) #[cfg(unix)];
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
    }
}

pub const CLEAN_EXIT: i32 = 0;
