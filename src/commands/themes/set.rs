use crate::core::*;

use crate::*;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  let theme = args.value_of("THEME").unwrap();
  let file_path = ctx.root.join("themes").join(theme);
  if !file_path.is_file() {
    return Err(CliError::Cmd {
      message: format!("no such theme \"{}\"", theme),
    });
  }

  let _ =
    fs::copy(file_path, ctx.root.join("current-theme")).unwrap();

  ctx
    .printer
    .plain(format!("Current theme is now \"{}\"", theme).as_str());

  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("set")
      .arg(Arg::with_name("THEME").help("Theme name").required(true))
      .about("Set current theme"),
  )
}
