use crate::*;

mod root;
mod task;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  match args.subcommand() {
    ("task", Some(m)) => task::exec(ctx, m),
    _ => root::exec(ctx, args),
  }
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  let sub = SubCommand::with_name("move")
    .setting(AppSettings::ArgRequiredElseHelp)
    .alias("mv")
    .about("Moves task or interval");
  let sub = root::register(sub);
  let sub = task::register(sub);

  app.subcommand(sub)
}
