use yatt_orm::DBError;

use crate::{core::Node, *};

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  let (ids, tags) = parse_args(args)?;

  let mut updated: Vec<Node> = Vec::new();

  for id in ids {
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

    node.add_tags(&tags);
    ctx.db.save(&node)?;
    updated.push(node);
  }

  if !updated.is_empty() {
    let mut cmd_text = "Tags updated";
    for node in updated {
      ctx.printer.node_cmd(&NodeCmdData {
        cmd_text,
        node: NodeData {
          node: &[node],
          title: "",
        },
      });
      ctx.printer.plain("");
      if !cmd_text.is_empty() {
        cmd_text = "";
      }
    }
  }

  Ok(())
}

pub(crate) fn parse_args(
  args: &ArgMatches,
) -> CliResult<(Vec<usize>, Vec<String>)> {
  let id = args.value_of("ID").unwrap().to_lowercase();
  let ids: Vec<usize> = if id == "cur" || id == "current" {
    vec![0]
  } else {
    let parsed: Vec<Result<_, _>> = id
      .split(',')
      .map(|v| v.trim())
      .filter(|v| !v.is_empty())
      .map(|v| v.parse())
      .collect();
    if parsed.iter().any(|v| v.is_err()) {
      return Err(CliError::Parse {
        message: "Unable to parse task ID".into(),
      });
    }
    let mut res = Vec::new();

    for v in parsed {
      res.push(v.unwrap());
    }

    res
  };
  let tags: Vec<String> = args
    .value_of("TAGS")
    .unwrap()
    .split(',')
    .map(|s| s.trim().to_lowercase())
    .filter(|s| !s.is_empty())
    .collect();
  Ok((ids, tags))
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("tag")
      .about("Adds comma separated tags to a task")
      .setting(AppSettings::ArgRequiredElseHelp)
      .arg(
        Arg::with_name("ID")
          .help("Comma separated task ids\nor \"cur[rent]\" (for current running task)")
          .required(true),
      )
      .arg(
        Arg::with_name("TAGS")
          .help("Comma separated tags list")
          .required(true),
      ),
  )
}
