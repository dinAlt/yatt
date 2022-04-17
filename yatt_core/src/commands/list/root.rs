use crate::*;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  _ctx: &AppContext<T, P>,
  _args: &ArgMatches,
) -> CliResult<()> {
  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app
}
