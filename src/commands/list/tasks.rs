use crate::core::*;
use crate::*;
use std::mem;
use trees::ForestWalk;
use yatt_orm::statement::eq;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
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
  if args.is_present("groups") {
    let mut v = walk_iter
      .filter(|r| r.len() > 1)
      .map(|mut r| {
        r.pop();
        r
      })
      .collect::<Vec<_>>();
    v.sort();
    v.dedup();
    ctx.printer.task_list(v.into_iter());
  } else {
    ctx.printer.task_list(walk_iter);
  }

  // XXX: ForestWalk panics on drop, so ¯\_(ツ)_/¯
  mem::forget(walk);

  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("tasks")
      .about("Shows tasks list")
      .arg(
        Arg::with_name("groups")
          .help("Show only tasks with children")
          .short("g")
          .long("groups"),
      ),
  )
}
