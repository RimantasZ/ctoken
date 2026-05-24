use crate::format::fmt_verbose_line;
use crate::tokenize_files::FileResult;

pub fn print_verbose(results: &[FileResult]) {
    for r in results {
        println!("{}", fmt_verbose_line(&r.entry.rel, &r.outcome));
    }
}
