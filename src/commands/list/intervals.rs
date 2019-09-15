use crate::*;

pub(crate) fn exec(_ctx: &AppContext, _args: &ArgMatches) -> CliResult<()> {
    Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
    app.subcommand(
        SubCommand::with_name("intervals")
            .setting(AppSettings::AllowNegativeNumbers)
            .about("Show intervals list"),
    )
}
