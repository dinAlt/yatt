use crate::{core::Interval, *};
use yatt_orm::statement::*;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  let task_id: usize = if args.is_present("task") {
    args.value_of("task").unwrap().parse().map_err(|_| {
      CliError::Parse {
        message: "Unable to parse task ID".into(),
      }
    })?
  } else {
    0
  };

  let filters = if task_id > 0 {
    and(
      eq(Interval::node_id_n(), task_id),
      eq(Interval::deleted_n(), 0),
    )
  } else {
    eq(Interval::deleted_n(), 0)
  };

  let intervals: Vec<Interval> = ctx.db.get_by_statement(
    filter(filters)
      .sort(Interval::begin_n(), SortDir::Ascend)
      .sort(Interval::node_id_n(), SortDir::Ascend),
  )?;

  ctx.printer.interval_list(intervals.into_iter());

  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("intervals")
      .setting(AppSettings::AllowNegativeNumbers)
      .about("Show intervals list")
      .arg(
        Arg::with_name("task")
          .short("t")
          .long("task")
          .help("Filtered task ID")
          .takes_value(true)
          )
      // .arg(
      //   Arg::with_name("period")
      //     .short("p")
      //     .long("period")
      //     .help("Show only intervals that started in a given period")
      //     .takes_value(true)
      //     .multiple(true),
      // )
      // .arg(
      //   Arg::with_name("closed")
      //     .short("c")
      //     .long("closed")
      //     .help("Show intervals for closed tasks"),
      // )
      // .arg(
      //   Arg::with_name("deleted")
      //     .short("d")
      //     .long("deleted")
      //     .help("Show deleted intervals"),
      // )
      // .arg(
      //   Arg::with_name("active")
      //     .short("a")
      //     .long("active")
      //     .help("Show active intervals (default)"),
      // ),
  )
}
