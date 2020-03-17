use crate::*;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  _ctx: &AppContext<T, P>,
  _args: &ArgMatches,
) -> CliResult<()> {
  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("intervals")
      .setting(AppSettings::AllowNegativeNumbers)
      .about("Show intervals list")
      .arg(
        Arg::with_name("period")
          .short("p")
          .long("period")
          .help("Show only intervals that started in a given period")
          .takes_value(true)
          .multiple(true),
      )
      .arg(
        Arg::with_name("closed")
          .short("c")
          .long("closed")
          .help("Show intervals for closed tasks"),
      )
      .arg(
        Arg::with_name("deleted")
          .short("d")
          .long("deleted")
          .help("Show deleted intervals"),
      )
      .arg(
        Arg::with_name("active")
          .short("a")
          .long("active")
          .help("Show active intervals (default)"),
      ),
  )
}
