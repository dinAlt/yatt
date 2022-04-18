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

  let interval = match interval {
    None => {
      let end = Utc::now();
      let begin: DateTime<Utc> =
        if let Some(b) = (end - duration).into() {
          b
        } else {
          return Err(CliError::Unexpected {
            message: "Can't calculate new interval duration".into(),
          });
        };
      Interval {
        begin,
        end: Some(end),
        closed: false,
        deleted: false,
        id: 0,
        node_id: Some(node.id),
      }
    }
    Some(interval) => {
      if let Some(end) = interval.end {
        let now = Utc::now();
        if now - end >= duration {
          Interval {
            begin: interval.begin,
            end: Some(end + duration),
            closed: false,
            deleted: false,
            id: interval.id,
            node_id: interval.node_id,
          }
        } else {
          let rest = duration - (now - end);
          let end = now;
          let begin: DateTime<Utc> = interval.begin - rest;

          if !ctx
            .db
            .get_by_statement::<Interval>(
              filter(and(
                gt(Interval::end_n(), begin),
                eq(Interval::deleted_n(), 0),
              ))
              .limit(1),
            )?
            .is_empty()
          {
            return Err(CliError::Cmd {
              message: "Given interval is too long".into(),
            });
          }

          Interval {
            begin,
            end: Some(end),
            closed: false,
            deleted: false,
            id: interval.id,
            node_id: interval.node_id,
          }
        }
      } else {
        let begin: DateTime<Utc> = interval.begin - duration;
        if !ctx
          .db
          .get_by_statement::<Interval>(
            filter(and(
              gt(Interval::end_n(), begin),
              eq(Interval::deleted_n(), 0),
            ))
            .limit(1),
          )?
          .is_empty()
        {
          return Err(CliError::Cmd {
            message: "Given interval is too long".into(),
          });
        }

        Interval {
          begin,
          end: interval.end,
          closed: false,
          deleted: false,
          id: interval.id,
          node_id: interval.node_id,
        }
      }
    }
  };

  let cmd_text = if interval.id > 0 {
    &"Interval extended"
  } else {
    &"New interval created"
  };

  ctx
    .db
    .save(&interval)
    .map_err(|e| CliError::DB { source: e })?;

  let nodes = ctx.db.ancestors(node.id)?;

  ctx.printer.interval_cmd(&IntervalCmdData {
    cmd_text,
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
    SubCommand::with_name("add")
      .about(
        "Adds time to or creates new interval\n\
                By default, gets task from last running interval",
      )
      .setting(AppSettings::ArgRequiredElseHelp)
      .arg(
        Arg::with_name("DURATION")
          .help("Time to add to interval")
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
