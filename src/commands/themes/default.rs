use crate::core::*;

use crate::*;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  _args: &ArgMatches,
) -> CliResult<()> {
  let file_path = ctx.root.join("current-theme");
  if file_path.is_file() {
    fs::remove_file(file_path).unwrap();
  }
  ctx.printer.plain("Using builtin theme");
  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("default").about("Restore builtin theme"),
  )
}
