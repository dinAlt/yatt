use super::*;

pub(crate) fn exec(ctx: &CmdContext, _ars: &ArgMatches) -> CliResult<()> {
    let res = ctx
        .db
        .cur_running()
        .map_err(|source| CliError::DB { source })?;

    let message = if let Some((node, interval)) = res {
        format!(
            r#"Task *"{}"* (started at **{}**)."#,
            node.label,
            format_datetime(&interval.begin)
        )
    } else {
        r#"there is no task running"#.to_string()
    };

    ctx.app.skin.print_text(&message);
    Ok(())
}
