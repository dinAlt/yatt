use crate::*;

mod restart;
mod root;
mod start;
mod state;
mod stop;
mod reports;

pub fn exec(ctx: &AppContext) -> CliResult<()> {
    match ctx.args.subcommand() {
        ("start", Some(m)) => start::exec(ctx, m),
        ("stop", Some(m)) => stop::exec(ctx, m),
        ("restart", Some(m)) => restart::exec(ctx, m),
        ("state", Some(m)) => state::exec(ctx, m),
        ("report", Some(m)) => reports::exec(ctx, m),
        _ => root::exec(ctx, &ctx.args),
    }
}