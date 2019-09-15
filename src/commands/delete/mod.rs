use crate::*;

mod interval;
mod root;

pub(crate) fn exec(ctx: &AppContext, args: &ArgMatches) -> CliResult<()> {
    match args.subcommand() {
        ("interval", Some(m)) => interval::exec(ctx, m),
        _ => root::exec(ctx, args),
    }
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
    let sub = SubCommand::with_name("delete")
        .setting(AppSettings::ArgRequiredElseHelp)
        .alias("remove")
        .alias("rm")
        .about("Delete entity")
        .arg(
            Arg::with_name("yes")
                .short("y")
                .help("Delete with no prompt"),
        );
    let sub = root::register(sub);
    let sub = interval::register(sub);

    app.subcommand(sub)
}
