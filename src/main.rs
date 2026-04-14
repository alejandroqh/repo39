mod changes;
mod config;
mod deps;
mod git;
mod glob;
mod identify;
mod map;
mod util;
mod walk;

use std::collections::HashSet;
use std::io::{self, BufWriter, Write};

use clap::Parser;

use config::{Cli, InfoFlags, ShowFilter, SizeUnit, SortOrder};
use changes::run_changes;
use deps::run_deps;
use git::load_git_dirty;
use glob::Glob;
use identify::run_identify;
use map::run_map;
use util::canonicalize;
use walk::{grep_walk, walk, WalkCtx};

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let mut dir_buf = canonicalize(&cli.path)?;

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let standalone_count = [cli.identify, cli.map, cli.deps, cli.changes]
        .iter().filter(|&&b| b).count();

    if standalone_count > 0 {
        let multi = standalone_count > 1;
        let mut sect = 0u8;

        macro_rules! section {
            ($flag:expr, $label:expr, $run:expr) => {
                if $flag {
                    if multi {
                        if sect > 0 { writeln!(out)?; }
                        writeln!(out, "[{}]", $label)?;
                    }
                    $run(&dir_buf, &mut out)?;
                    sect += 1;
                }
            };
        }

        section!(cli.identify, "identify", |root: &std::path::Path, out: &mut _| run_identify(root, out));
        section!(cli.deps, "deps", |root: &std::path::Path, out: &mut _| run_deps(root, out));
        section!(cli.map, "map", |root: &std::path::Path, out: &mut _| run_map(root, cli.depth, cli.limit, cli.grep.as_deref(), out));
        section!(cli.changes, "changes", |root: &std::path::Path, out: &mut _| run_changes(root, out));
        let _ = sect;

        return Ok(());
    }

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

    if let Some(ref pattern) = cli.grep {
        let glob = Glob::compile(pattern);
        grep_walk(&mut dir_buf, &ctx, &glob, 0, &mut out)?;
    } else {
        walk(&mut dir_buf, &ctx, 0, &mut out)?;
    }

    Ok(())
}
