use crate::*;

mod add;
mod cancel;
mod delete;
mod list;
mod reports;
mod restart;
mod root;
mod start;
mod state;
mod stop;

pub fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
) -> CliResult<()> {
  match ctx.args.subcommand() {
    ("start", Some(m)) => start::exec(ctx, m),
    ("stop", Some(m)) => stop::exec(ctx, m),
    ("restart", Some(m)) => restart::exec(ctx, m),
    ("state", Some(m)) => state::exec(ctx, m),
    ("report", Some(m)) => reports::exec(ctx, m),
    ("cancel", Some(m)) => cancel::exec(ctx, m),
    ("delete", Some(m)) => delete::exec(ctx, m),
    ("list", Some(m)) => list::exec(ctx, m),
    ("add", Some(m)) => add::exec(ctx, m),
    _ => root::exec(ctx, &ctx.args),
  }
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  let app = root::register(app);
  let app = start::register(app);
  let app = stop::register(app);
  let app = restart::register(app);
  let app = state::register(app);
  let app = cancel::register(app);
  let app = reports::register(app);
  let app = list::register(app);
  let app = add::register(app);

  delete::register(app)
}
