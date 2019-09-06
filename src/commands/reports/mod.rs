use crate::*;

mod root;

pub(crate) fn exec(ctx: &AppContext, args: &ArgMatches) -> CliResult<()> {
    match ctx.args.subcommand() {
        _ => root::exec(ctx, args),
    }
}
