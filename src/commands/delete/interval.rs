use crate::core::*;
use crate::*;
use crossterm_input::input;
use std::convert::TryInto;
use yatt_orm::{statement::*, DBError, FieldVal};

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  let no_prompt = args.is_present("yes");
  let id: i64 =
    args.value_of("ID").unwrap().parse().map_err(|_| {
      CliError::Parse {
        message: "Unable to parse interval ID".into(),
      }
    })?;

  let mut interval = if id < 0 {
    let offset: usize = (-id).try_into().unwrap();
    let intervals: Vec<Interval> = ctx.db.get_by_statement(
      filter(and(
        ne(Interval::deleted_n(), 1),
        ne(Interval::end_n(), FieldVal::Null),
      ))
      .sort(&Interval::begin_n(), SortDir::Descend)
      .limit(offset),
    )?;
    if intervals.len() < offset {
      return Err(CliError::Cmd {
        message: "There is no interval with given offset".into(),
      });
    }
    intervals.last().unwrap().to_owned()
  } else {
    let id: usize = id.try_into().unwrap();
    ctx.db.get_by_id::<Interval>(id).map_err(|source| {
      if let DBError::IsEmpty { .. } = source {
        return CliError::Cmd {
          message: "There is no interval with given ID".into(),
        };
      }

      CliError::DB { source }
    })?
  };

  let task: Node = ctx
    .db
    .get_by_id(interval.node_id.unwrap())
    .map_err(|source| CliError::DB { source })?;
  let task = ctx
    .db
    .ancestors(task.id)
    .map_err(|source| CliError::DB { source })?;

  interval.deleted = true;
  let interval_data = IntervalData {
    interval: &interval,
    title: IntervalData::default_title(),
    task: &task,
  };

  if no_prompt {
    ctx
      .db
      .save(&interval)
      .map_err(|source| CliError::DB { source })?;

    ctx.printer.interval_cmd(&IntervalCmdData {
      cmd_text: &"Successfully deleted...",
      interval: interval_data,
    });
  } else {
    ctx.printer.interval_cmd(&IntervalCmdData {
      cmd_text: &"Are you sure, you want to delete interval? [y/n]",
      interval: interval_data,
    });
    let input = input();
    if input.read_char()?.to_lowercase().to_string() == "y" {
      ctx
        .db
        .save(&interval)
        .map_err(|source| CliError::DB { source })?;
      ctx.printer.cmd("Successfully deleted...")
    } else {
      ctx.printer.cmd("Cancelled...")
    }
  }

  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("interval")
      .setting(AppSettings::AllowNegativeNumbers)
      .about("Deletes an interval")
      .arg(
        Arg::with_name("ID")
          .help("[ID] or -[offset] from NOW (starting with -1)")
          .required(true),
      ),
  )
}
