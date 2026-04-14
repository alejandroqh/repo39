mod config;
mod git;
mod glob;
mod util;
mod walk;

use std::collections::HashSet;
use std::io::{self, BufWriter};

use clap::Parser;

use config::{Cli, InfoFlags, ShowFilter, SizeUnit, SortOrder};
use git::load_git_dirty;
use glob::Glob;
use util::canonicalize;
use walk::{grep_walk, walk, WalkCtx};

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let mut dir_buf = canonicalize(&cli.path)?;
    let filter = ShowFilter::parse(&cli.show, cli.depth);
    let order = SortOrder::parse(&cli.order);
    let info = InfoFlags::parse(&cli.info, order);

    let dirty_files = if info.git {
        load_git_dirty(&dir_buf, true)
    } else {
        HashSet::new()
    };

    let ctx = WalkCtx {
        root: &dir_buf.clone(),
        filter,
        order,
        info,
        unit: SizeUnit::parse(&cli.unit),
        limit: cli.limit,
        dirty_files,
    };

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    if let Some(ref pattern) = cli.grep {
        let glob = Glob::compile(pattern);
        grep_walk(&mut dir_buf, &ctx, &glob, 0, &mut out)?;
    } else {
        walk(&mut dir_buf, &ctx, 0, &mut out)?;
    }

    Ok(())
}
