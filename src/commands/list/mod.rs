use crate::*;

mod intervals;
mod tasks;

pub(crate) fn exec<T: DBRoot, P: Printer>(
    ctx: &AppContext<T, P>,
    args: &ArgMatches,
) -> CliResult<()> {
    match args.subcommand() {
        ("tasks", Some(m)) => tasks::exec(ctx, m),
        ("intervals", Some(m)) => intervals::exec(ctx, m),
        _ => root::exec(ctx, args),
    }
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
    let sub = SubCommand::with_name("list")
        .setting(AppSettings::ArgRequiredElseHelp)
        .about("List entities");
    let sub = tasks::register(sub);
    let sub = intervals::register(sub);

    app.subcommand(sub)
}
