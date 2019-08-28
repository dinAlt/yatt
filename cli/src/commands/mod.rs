use super::*;
use sqlite::DB;

mod restart;
mod root;
mod start;
mod state;
mod stop;

pub fn exec(ctx: AppContext) -> CliResult<()> {
    let db = {
        match DB::new(ctx.root.join(&ctx.conf.db_path)) {
            Ok(db) => db,
            Err(e) => return Err(CliError::DB { source: e }),
        }
    };

    let c_ctx = CmdContext {
        app: ctx,
        db: Box::new(db),
    };

    select_and_exec(&c_ctx)
}

pub(crate) struct CmdContext<'a> {
    pub app: AppContext<'a>,
    pub db: Box<dyn core::DBRoot>,
}

fn select_and_exec(ctx: &CmdContext) -> CliResult<()> {
    if let Some(m) = ctx.app.args.subcommand_matches("start") {
        return start::exec(ctx, m);
    }
    if let Some(m) = ctx.app.args.subcommand_matches("stop") {
        return stop::exec(ctx, m);
    }
    if let Some(m) = ctx.app.args.subcommand_matches("restart") {
        return restart::exec(ctx, m);
    }
    if let Some(m) = ctx.app.args.subcommand_matches("state") {
        return state::exec(ctx, m);
    }
    root::exec(ctx, &ctx.app.args)
}
