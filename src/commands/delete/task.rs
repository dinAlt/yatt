use crate::core::*;
use crate::*;
use crossterm_input::input;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  let no_prompt = args.is_present("yes");
  let id: usize =
    args.value_of("ID").unwrap().parse().map_err(|_| {
      CliError::Parse {
        message: "Unable to parse task ID".into(),
      }
    })?;
  let mut task = ctx.db.get_by_id::<Node>(id)?;
  let is_group = ctx.db.has_children(task.id)?;

  if task.deleted {
    return Err(CliError::Cmd {
      message: "Task already deleted".into(),
    });
  }

  if let Some((node, _)) = ctx.db.cur_running()? {
    if node.id == id {
      return Err(CliError::Cmd {
        message: "Can't delete current running task. \
          Stop the task and try again, please."
          .into(),
      });
    }
    if is_group {
      return Err(CliError::Cmd {
        message: "Can't delete task with children \
          while an interval is running. \
          Stop the task and try again, please."
          .into(),
      });
    }
  }

  task.deleted = true;
  let path = ctx.db.ancestors(task.id)?;
  let node_data = NodeData {
    title: NodeData::default_title(),
    node: &path,
  };

  let cmd_text = if is_group {
    "Selected task has children. \
      Are you sure, you want to delete it \
      with all it's children and intervals? [y/n]"
  } else {
    "Are you sure, you want to delete \
      task and all it's intervals? [y/n]"
  };

  if !no_prompt {
    ctx.printer.node_cmd(&NodeCmdData {
      cmd_text,
      node: node_data,
    });
    let input = input();
    if input
      .read_char()
      .map_err(|e| CliError::wrap(Box::new(e)))
      .unwrap_or_default()
      .to_string()
      != "y"
    {
      ctx.printer.cmd("Cancelled...");
      return Ok(());
    }
  }

  ctx.db.remove_node(task.id)?;
  ctx.printer.cmd("Successfully deleted...");

  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("task")
      .setting(AppSettings::AllowNegativeNumbers)
      .about("Deletes a task")
      .arg(Arg::with_name("ID").help("Task id").required(true)),
  )
}
