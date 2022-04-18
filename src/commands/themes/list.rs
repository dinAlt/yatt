use crate::core::*;

use crate::*;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  _args: &ArgMatches,
) -> CliResult<()> {
  let themes_dir: PathBuf = ctx.root.join("themes");
  if !themes_dir.is_dir() {
    ctx.printer.plain("There is no themes installed.");
    return Ok(());
  }

  print_themes_list(&themes_dir, ctx)
}

fn print_themes_list<T: DBRoot, P: Printer>(
  themes_dir: &Path,
  ctx: &AppContext<T, P>,
) -> CliResult<()> {
  ctx.printer.theme_list(
    fs::read_dir(themes_dir)
      .map_err(|source| CliError::wrap(Box::new(source)))?
      .filter_map(|d| {
        let d = d.unwrap();
        if d.file_type().unwrap().is_file() {
          Some(ThemeData {
            title: d.file_name().into_string().unwrap(),
            theme: Theme::try_from(
              fs::read_to_string(d.path()).unwrap().as_str().trim(),
            )
            .unwrap(),
          })
        } else {
          None
        }
      }),
  );

  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("list").about("List installed themes"),
  )
}
