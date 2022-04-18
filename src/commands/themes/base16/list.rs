use crate::base16::get_themes_list;
use crate::*;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  _args: &ArgMatches,
) -> CliResult<()> {
  let themes = get_themes_list()?;

  for (k, _) in themes {
    ctx.printer.plain(format!("   {}", k).as_str());
  }

  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("list")
      .about("List available theme sources")
      .long_about("List sources from https://github.com/chriskempson/base16-templates-source"),
  )
}
