use crate::*;

mod interval;

pub(crate) fn exec(ctx: &AppContext, args: &ArgMatches) -> CliResult<()> {
    match args.subcommand() {
        ("interval", Some(m)) => interval::exec(ctx, m),
        _ => root::exec(ctx, args),
    }
}
