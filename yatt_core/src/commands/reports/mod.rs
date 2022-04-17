use crate::*;

mod root;
mod total;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  match args.subcommand() {
    ("total", Some(m)) => total::exec(ctx, m),
    _ => root::exec(ctx, args),
  }
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  let sub = SubCommand::with_name("report")
    .setting(AppSettings::ArgRequiredElseHelp)
    .about("Shows selected report");
  let sub = root::register(sub);
  let sub = total::register(sub);

  app.subcommand(sub)
}
