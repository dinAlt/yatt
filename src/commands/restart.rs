use crate::core::{Interval, Node};
use crate::*;
use yatt_orm::statement::*;

#[allow(clippy::field_reassign_with_default)]
pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  let res = ctx
    .db
    .cur_running()
    .map_err(|source| CliError::DB { source })?;

  if let Some((node, interval)) = res {
    let task = ctx.db.ancestors(node.id)?;
    return Err(CliError::Task {
      source: TaskError::CmdTaskInterval {
        message: "Interval already running.".to_string(),
        interval,
        task,
      },
    });
  };

  let node_id = if args.is_present("ID") {
    args.value_of("ID").unwrap().parse().map_err(|_| {
      CliError::Parse {
        message: "Unable to parse task ID".into(),
      }
    })?
  } else {
    let interval: Vec<Interval> = ctx.db.get_by_statement(
      filter(and(
        ne(Interval::deleted_n(), 1),
        ne(Interval::closed_n(), 1),
      ))
      .sort(Interval::end_n(), SortDir::Descend)
      .limit(1),
    )?;

    if interval.is_empty() {
      return Err(CliError::Task {
        source: TaskError::Cmd {
          message: "There is no priviosly started tasks.".to_string(),
        },
      });
    }
    interval.first().unwrap().to_owned().node_id.unwrap()
  };

  let mut interval = Interval::default();
  interval.node_id = Some(node_id);

  let node: Vec<Node> = ctx
    .db
    .get_by_filter(eq(Node::id_n(), interval.node_id.unwrap()))?;
  if node.is_empty() {
    return Err(CliError::Unexpected {
      message: format!(
        "node with id {} not found",
        interval.node_id.unwrap()
      ),
    });
  };

  let node = node.first().unwrap();

  ctx.db.save(&interval)?;

  let task = &ctx.db.ancestors(node.id)?;
  ctx.printer.interval_cmd(&IntervalCmdData {
    cmd_text: "Restarting...",
    interval: IntervalData {
      interval: &interval,
      task,
      title: IntervalData::default_title(),
    },
  });

  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("restart")
      .about("Restarts last task")
      .arg(
        Arg::with_name("ID")
          .help("Task id. By default, the last runned task is used"),
      ),
  )
}
