use crate::core::*;
use crate::*;
use std::mem;
use trees::ForestWalk;
use yatt_orm::statement::eq;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  _args: &ArgMatches,
) -> CliResult<()> {
  let forest =
    ctx.db.get_filtered_forest(eq(Node::deleted_n(), 0))?;
  let forest = if forest.is_some() {
    forest.unwrap()
  } else {
    return Ok(());
  };
  let mut walk = ForestWalk::from(forest);
  let walk_iter = FlattenForestIter::new(&mut walk);

  ctx.printer.task_list(walk_iter);

  // TODO: ForestWalk panics on drop, so ¯\_(ツ)_/¯
  mem::forget(walk);

  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("tasks")
      .setting(AppSettings::AllowNegativeNumbers)
      .about("Shows tasks list"),
  )
}
