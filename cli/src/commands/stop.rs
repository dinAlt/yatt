use super::*;

pub(crate) fn exec(ctx: &CmdContext, _ars: &ArgMatches) -> CliResult<()> {
    let res = ctx
        .db
        .cur_running()
        .map_err(|source| CliError::DB { source })?;
    let (node, mut interval) = match res {
        Some((n, i)) => (n, i),
        None => {
            return Err(CliError::Cmd {
                message: r#"there is no task running"#.to_string(),
            })
        }
    };

    interval.end = Some(Utc::now());
    ctx.db.intervals().save(&interval)?;
    let text = format!(
        r#"Task *"{}"* started at **{}** has been stopped just now."#,
        node.label,
        format_datetime(&interval.begin)
    );

    ctx.app.skin.print_text(&text);

    Ok(())
}
