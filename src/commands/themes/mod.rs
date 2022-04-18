use crate::*;

mod default;
mod list;
mod root;
mod set;

#[cfg(feature = "base16")]
mod base16;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  match args.subcommand() {
    ("list", Some(m)) => list::exec(ctx, m),
    ("set", Some(m)) => set::exec(ctx, m),
    ("default", Some(m)) => default::exec(ctx, m),
    #[cfg(feature = "base16")]
    ("base16", Some(m)) => base16::exec(ctx, m),
    _ => root::exec(ctx, args),
  }
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  let sub = SubCommand::with_name("themes")
    .setting(AppSettings::ArgRequiredElseHelp)
    .about("Themes managing");
  let sub = root::register(sub);
  let sub = list::register(sub);
  let sub = set::register(sub);
  let sub = default::register(sub);

  #[cfg(feature = "base16")]
  let sub = base16::register(sub);

  app.subcommand(sub)
}
