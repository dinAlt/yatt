use crate::*;

mod root;
mod total;

pub(crate) fn exec(ctx: &AppContext, args: &ArgMatches) -> CliResult<()> {
    match args.subcommand() {
        ("total", Some(m)) => total::exec(ctx, m),
        _ => root::exec(ctx, args),
    }
}
