use std::collections::HashMap;
use std::io::Write;

use crate::base16::*;
use crate::*;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  ctx.printer.plain("Getting sources list...");
  ctx.printer.plain("");
  let list = get_themes_list()?;
  let sources: Vec<&str> = if args.is_present("all") {
    list.keys().map(|k| k.as_str()).collect()
  } else {
    args.value_of("THEMES").unwrap_or("").split(',').collect()
  };
  if sources.is_empty() {
    ctx.printer.error("nothing to import");
    return Err(CliError::Cmd {
      message: "nothing to import".into(),
    });
  }

  let mut not_founds = Vec::new();
  for source in sources.iter() {
    if list.get(*source).is_none() {
      not_founds.push(*source);
    }
  }
  if !not_founds.is_empty() {
    return Err(CliError::Cmd {
      message: format!(
        "sources not found:\n   {}\n",
        not_founds.join("\n   ")
      ),
    });
  }

  let mut has_errors = false;
  let mut themes: HashMap<String, Base16> = HashMap::new();

  for source in sources {
    ctx
      .printer
      .plain(format!("Downloading {}", source).as_str());

    match get_theme_zip(list.get(source).unwrap()) {
      Ok(zip_file) => match get_themes_from_zip(zip_file) {
        Ok(theme_results) => {
          for (k, v) in theme_results {
            match v {
              Ok(b16theme) => {
                themes.insert(k, b16theme);
              }
              Err(e) => {
                has_errors = true;
                ctx.printer.error(format!("{}", e).as_str());
              }
            }
          }
        }
        Err(e) => {
          has_errors = true;
          ctx.printer.error(format!("{}", e).as_str());
        }
      },
      Err(e) => {
        has_errors = true;
        ctx.printer.error(format!("{}", e).as_str());
      }
    }
  }

  let themes_dir = ctx.root.join("themes");
  fs::create_dir_all(&themes_dir).unwrap();

  let mut print_list: Vec<ThemeData> = Vec::new();

  for (k, v) in convert_themes(&themes) {
    let mut theme_file =
      fs::File::create(themes_dir.join(&k)).unwrap();

    let str_theme: String = v.clone().into();
    theme_file.write_all(str_theme.as_bytes()).unwrap();

    print_list.push(ThemeData { title: k, theme: v });
  }

  if !themes.is_empty() {
    ctx.printer.plain("");
    ctx.printer.plain("Themes imported:");
    ctx.printer.plain("");
    ctx.printer.theme_list(print_list.into_iter());
  }

  if has_errors {
    return Err(CliError::Cmd {
      message: "import completed with errors".into(),
    });
  }
  Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("import")
      .about("Import themes from sources (use \"list\" to find available sources)")
      .setting(AppSettings::ArgRequiredElseHelp)
      .arg(
        Arg::with_name("all")
        .long("all")
        .short("a")
        .help("download and import all themes")
        .required(false))
      .arg(
        Arg::with_name("THEMES")
          .help("Comma separated list of theme sources")
          .required(false),
      ),
  )
}
