use crate::{core::*, parse::*, *};
use yatt_orm::statement::*;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  let duration = parse_duration(args.value_of("DURATION").unwrap())?;
  let task: Option<usize> = if let Some(t) = args.value_of("task") {
    Some(t.parse().map_err(|e| CliError::wrap(Box::new(e)))?)
  } else {
    None
  };

  let (node, interval) = if let Some(id) = task {
    let node: Node = ctx.db.get_by_id(id)?;
    let mut interval: Vec<Interval> = ctx.db.get_by_statement(
      filter(and(
        eq(Interval::id_n(), node.id),
        eq(Interval::deleted_n(), 0),
      ))
      .limit(1),
    )?;
    if interval.is_empty() {
      (node, None)
    } else {
      (node, Some(interval.pop().unwrap()))
    }
  } else if let Some((node, interval)) = ctx.db.cur_running()? {
    (node, Some(interval))
  } else if let Some((node, interval)) = ctx.db.last_running()? {
    (node, Some(interval))
  } else {
    return Err(CliError::Cmd {
      message: "There is no previous running task".into(),
    });
  };

  if interval.is_none() {
    return Err(CliError::Cmd {
      message: "There is no matched interval".into(),
    });
  }

  let interval = interval.unwrap();
  if interval.end.is_none() {
    return Err(CliError::Cmd {
      message: "Unable truncate running interval".into(),
    });
  }

  let interval_duration = interval.end.unwrap() - interval.begin;
  if duration > interval_duration {
    return Err(CliError::Cmd {
      message: "Matched interval is too short".into(),
    });
  }

  let interval = Interval {
    id: interval.id,
    begin: interval.begin,
    end: Some(interval.end.unwrap() - duration),
    deleted: false,
    closed: false,
    node_id: interval.node_id,
  };

  ctx
    .db
    .save(&interval)
    .map_err(|e| CliError::DB { source: e })?;

  let nodes = ctx.db.ancestors(node.id)?;

  ctx.printer.interval_cmd(&IntervalCmdData {
    cmd_text: "Interval is truncated",
    interval: IntervalData {
      interval: &interval,
      task: &nodes,
      title: IntervalData::default_title(),
    },
  });

  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("trunc")
      .alias("truncate")
      .about(
        "Truncates interval for a given duration\n\
                By default, truncates last running interval",
      )
      .setting(AppSettings::ArgRequiredElseHelp)
      .arg(
        Arg::with_name("DURATION")
          .help("Truncate duration")
          .required(true),
      )
      .arg(
        Arg::with_name("task")
          .short("t")
          .long("task")
          .help("Task id")
          .takes_value(true)
          .multiple(true),
      ),
  )
}
