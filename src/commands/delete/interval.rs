use crate::*;

pub(crate) fn exec(ctx: &AppContext, _args: &ArgMatches) -> CliResult<()> {
    // let res = ctx
    //     .db
    //     .cur_running()
    //     .map_err(|source| CliError::DB { source })?;

    // if res.is_none() {
    //     return Err(CliError::Task{source: TaskError::Cmd{
    //         message: "No task running.".to_string(),
    //     }});
    // }

    // let (node, mut interval) = res.unwrap();

    // interval.end = Some(Utc::now());
    // ctx.db.intervals().save(&interval)?;

    // let task = &ctx.db.ancestors(node.id)?;

    // ctx.printer.interval_cmd(&IntervalCmdData {
    //     cmd_text: "Stopping...",
    //     interval: IntervalData {
    //         interval: &interval,
    //         task,
    //         title: IntervalData::default_title(),
    //     },
    // });

    Ok(())
}
