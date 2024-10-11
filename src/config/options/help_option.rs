use super::Options;
use anyhow::{anyhow, Context, Result};
use clap::{builder::StyledStr, Arg, CommandFactory};

fn arg_get_help(arg: &Arg) -> Result<StyledStr> {
    arg.get_long_help().map_or_else(
        || {
            arg.get_help().map_or_else(
                || {
                    Err(anyhow!(
                        "Error: failed to find help for \"{}\" argument",
                        arg.get_id()
                    ))
                },
                |help| Ok(help.clone()),
            )
        },
        |help| Ok(help.clone()),
    )
}

fn check_long_arg_names_and_aliases(string_raw: &str, command: &clap::Command) -> Result<()> {
    let mut string = string_raw.to_lowercase().replace('-', "_");
    if string.starts_with("__") {
        string.replace_range(.."__".len(), "");
    }
    match string.as_ref() {
        "help" => return Err(anyhow!("Print help (see a summary with '-h')")),
        "version" => return Err(anyhow!("Print version")),
        _ => {
            for arg in command.get_arguments() {
                if arg.get_id() == &string {
                    return Err(anyhow!(arg_get_help(arg)?));
                } else if let Some(vec) = arg.get_all_aliases() {
                    for alias in vec {
                        if alias.to_lowercase().replace('-', "_") == string {
                            return Err(anyhow!(arg_get_help(arg)?));
                        }
                    }
                } else { //
                }
            }
        }
    };
    Ok(())
}

fn check_short_arg_names_and_aliases(string_raw: &str, command: &clap::Command) -> Result<()> {
    let string = string_raw
        .strip_prefix('-')
        .map_or_else(|| string_raw.to_owned(), ToOwned::to_owned);
    if string.len() == 1 {
        let character = string.chars().next().context("string is empty")?;
        match character {
            'h' => return Err(anyhow!("Print help (see more with '--help')")),
            'V' => return Err(anyhow!("Print version")),
            _ => {
                for arg in command.get_arguments() {
                    if let Some(short) = arg.get_short() {
                        if short == character {
                            return Err(anyhow!(arg_get_help(arg)?));
                        }
                    };
                    if let Some(vec) = arg.get_all_short_aliases() {
                        for alias in vec {
                            if alias == character {
                                return Err(anyhow!(arg_get_help(arg)?));
                            }
                        }
                    }
                }
            }
        }
    };
    Ok(())
}

pub(super) fn check_show_help_for_option(options: &Options) -> Result<()> {
    if let Some(ref string) = options.help_option {
        let command = Options::command();
        check_long_arg_names_and_aliases(string, &command)?;
        check_short_arg_names_and_aliases(string, &command)?;
        Err(anyhow!(
            "Failed to find option \"{}\" to show help for it. Use \"-h\" to get list of available options.",
            string
        ))
    } else {
        Ok(())
    }
}
