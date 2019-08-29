use super::*;

pub(crate) fn exec(_ctx: &AppContext, ars: &ArgMatches) -> CliResult<()> {
    println!("{}", ars.usage());
    Ok(())
}
