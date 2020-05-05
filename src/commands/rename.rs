use crate::*;

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
  let label: Vec<&str> = args.values_of("NAME").unwrap().collect();
  let label = label.join(" ");

  let mut path = ctx.db.ancestors(id)?;
  path.last_mut().unwrap().label = label;
  ctx.db.save(path.last().unwrap())?;

  ctx.printer.node_cmd(&NodeCmdData {
    cmd_text: "Successfully renamed.".into(),
    node: NodeData {
      title: NodeData::default_title(),
      node: &path,
    },
  });

  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("rename")
      .about("Renames a task")
      .setting(AppSettings::ArgRequiredElseHelp)
      .arg(Arg::with_name("ID").help("Task id").required(true))
      .arg(
        Arg::with_name("NAME")
          .help("New task name")
          .multiple(true)
          .required(true),
      ),
  )
}
