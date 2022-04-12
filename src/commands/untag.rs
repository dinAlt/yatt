use yatt_orm::DBError;

use crate::*;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  let (id, tags) = parse_args(args)?;

  let mut node = if id == 0 {
    if let Some((node, _)) = ctx.db.cur_running()? {
      node
    } else {
      return Err(CliError::Task {
        source: TaskError::Cmd {
          message: "No task running.".to_string(),
        },
      });
    }
  } else {
    ctx.db.get_by_id(id).map_err(|source| match source {
      DBError::IsEmpty { .. } => CliError::Task {
        source: TaskError::Cmd {
          message: "No task running".to_string(),
        },
      },
      _ => CliError::DB { source },
    })?
  };

  node.remove_tags(&tags);
  ctx.db.save(&node)?;
  ctx.printer.node_cmd(&NodeCmdData {
    cmd_text: "Tags updated",
    node: NodeData {
      node: &[node],
      title: "Task: ",
    },
  });

  Ok(())
}

pub(crate) fn parse_args(
  args: &ArgMatches,
) -> CliResult<(usize, Vec<String>)> {
  let id = args.value_of("ID").unwrap().to_lowercase();
  let id: usize = if id == "cur" || id == "current" {
    0
  } else {
    id.parse().map_err(|_| CliError::Parse {
      message: "Unable to parse task ID".into(),
    })?
  };
  let tags: Vec<String> = args
    .value_of("TAGS")
    .unwrap()
    .split(',')
    .map(|s| s.trim().to_lowercase())
    .filter(|s| !s.is_empty())
    .collect();
  Ok((id, tags))
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("untag")
      .about("Removes comma separated tags from task")
      .setting(AppSettings::ArgRequiredElseHelp)
      .arg(
        Arg::with_name("ID")
          .help("Task id or \"cur[rent]\" (for current running task)")
          .required(true),
      )
      .arg(
        Arg::with_name("TAGS")
          .help("Comma separated tags list")
          .required(true),
      ),
  )
}
