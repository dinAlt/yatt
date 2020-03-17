use crate::core::Interval;
use crate::*;

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

  let path: Vec<&str> = args.values_of("TASK").unwrap().collect();
  let path = path.join(" ");
  let path: Vec<&str> = path.split("::").map(|t| t.trim()).collect();

  let nodes = ctx.db.create_path(&path)?;
  let interval = Interval {
    id: 0,
    node_id: Some(nodes.last().unwrap().id),
    begin: Utc::now(),
    end: None,
    deleted: false,
    closed: false,
  };
  ctx.db.save(&interval)?;

  ctx.printer.interval_cmd(&IntervalCmdData {
    cmd_text: &"Starting...",
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
    SubCommand::with_name("start")
      .alias("run")
      .about("Starts new task, or continues existing")
      .setting(AppSettings::ArgRequiredElseHelp)
      .arg(
        Arg::with_name("TASK")
          .help("Task name with nested tasks, delimited by \"::\"")
          .required(true)
          .multiple(true),
      ),
  )
}
