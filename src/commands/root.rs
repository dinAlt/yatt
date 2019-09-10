use crate::*;

pub(crate) fn exec(_ctx: &AppContext, _args: &ArgMatches) -> CliResult<()> {
    Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
    app
}
