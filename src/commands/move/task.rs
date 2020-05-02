use crate::{core::Node, *};

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  let id: usize =
    args.value_of("ID").unwrap().parse().map_err(|_| {
      CliError::Parse {
        message: "Unable to parse task ID".into(),
      }
    })?;
  let parent_id: usize =
    args.value_of("PARENT_ID").unwrap().parse().map_err(|_| {
      CliError::Parse {
        message: "Unable to parse task ID".into(),
      }
    })?;

  if id == parent_id {
    return Err(CliError::Cmd {
      message: "It is impossible to move the task to itself".into(),
    });
  }

  let mut node: Node = ctx.db.get_by_id(id)?;

  let mut path = if parent_id > 0 {
    ctx.db.ancestors(parent_id)?
  } else {
    Vec::new()
  };

  node.parent_id = Some(parent_id);
  ctx.db.save(&node)?;
  path.push(node);

  ctx.printer.node_cmd(&NodeCmdData {
    cmd_text: "Successfully moved.".into(),
    node: NodeData {
      title: NodeData::default_title(),
      node: &path,
    },
  });

  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("task")
      .about("Moves task to new parent")
      .setting(AppSettings::ArgRequiredElseHelp)
      .arg(Arg::with_name("ID").help("Task id").required(true))
      .arg(
        Arg::with_name("PARENT_ID")
          .help("New parent id")
          .required(true),
      ),
  )
}
