use crate::*;

mod import;
mod list;
mod root;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  match args.subcommand() {
    ("list", Some(m)) => list::exec(ctx, m),
    ("import", Some(m)) => import::exec(ctx, m),
    (sub, _) => {
      println!("subcommand: {}", sub);
      root::exec(ctx, args)
    }
  }
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  let sub = SubCommand::with_name("base16")
    .setting(AppSettings::ArgRequiredElseHelp)
    .about("Base16 themes support");
  let sub = root::register(sub);
  let sub = list::register(sub);
  let sub = import::register(sub);

  app.subcommand(sub)
}
