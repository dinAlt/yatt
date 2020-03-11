use crate::*;

pub(crate) fn exec<T: DBRoot, P: Printer>(
    ctx: &AppContext<T, P>,
    _args: &ArgMatches,
) -> CliResult<()> {
    let res = ctx
        .db
        .cur_running()
        .map_err(|source| CliError::DB { source })?;

    if res.is_none() {
        return Err(CliError::Cmd {
            message: "no interval running.".into(),
        });
    }

    let (node, mut interval) = res.unwrap();
    let nodes = ctx.db.ancestors(node.id)?;
    interval.end = Some(Utc::now());
    interval.deleted = true;

    ctx.db.save(&interval)?;

    ctx.printer.interval_cmd(&IntervalCmdData {
        cmd_text: &"Current interval canceled...",
        interval: IntervalData {
            interval: &interval,
            task: &nodes,
            title: IntervalData::default_title(),
        },
    });

    Ok(())
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
    app.subcommand(
        SubCommand::with_name("cancel")
            .about("Cancels current interval."),
    )
}
