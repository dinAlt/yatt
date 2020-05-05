use crate::{core::Interval, *};
use yatt_orm::statement::*;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  let from_id: usize =
    args.value_of("FROM_ID").unwrap().parse().map_err(|_| {
      CliError::Parse {
        message: "Unable to parse FROM_ID".into(),
      }
    })?;
  let to_id: usize =
    args.value_of("TO_ID").unwrap().parse().map_err(|_| {
      CliError::Parse {
        message: "Unable to parse TO_ID".into(),
      }
    })?;

  let from_path = ctx.db.ancestors(from_id)?;
  let to_path = ctx.db.ancestors(to_id)?;

  for interval in ctx
    .db
    .get_by_filter::<Interval>(eq(
      Interval::node_id_n(),
      from_path.last().unwrap().id,
    ))?
    .iter_mut()
  {
    interval.node_id = Some(to_id);
    ctx.db.save(interval)?;
  }

  ctx.printer.node_cmd(&NodeCmdData {
    cmd_text: "Successfully merged.",
    node: NodeData {
      title: "From: ",
      node: &from_path,
    },
  });
  ctx.printer.node_cmd(&NodeCmdData {
    cmd_text: "",
    node: NodeData {
      title: "To:",
      node: &to_path,
    },
  });

  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("merge")
      .about("Moves all intervals from one task to another")
      .setting(AppSettings::ArgRequiredElseHelp)
      .arg(
        Arg::with_name("FROM_ID")
          .help("Merge from task id")
          .required(true),
      )
      .arg(
        Arg::with_name("TO_ID")
          .help("Merge to task id")
          .required(true),
      ),
  )
}
