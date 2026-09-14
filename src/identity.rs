use std::{env, io::{self, IsTerminal}};

const STACKPILOT_UNICODE: &str = include_str!("../identity/stackpilot-unicode.txt");
const STACKPILOT_ASCII: &str = include_str!("../identity/stackpilot-ascii.txt");

pub fn should_show_identity(args: &[String], is_terminal: bool, no_banner: bool) -> bool {
    if !is_terminal || no_banner {
        return false;
    }

    !args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--version" | "-V" | "--help" | "-h" | "--non-interactive"
        )
    })
}

pub fn print_if_interactive() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let no_banner = env::var("STACKPILOT_NO_BANNER").ok().as_deref() == Some("1");

    if !should_show_identity(&args, io::stdout().is_terminal(), no_banner) {
        return;
    }

    let ascii = env::var("STACKPILOT_ASCII").ok().as_deref() == Some("1");
    let banner = if ascii {
        STACKPILOT_ASCII
    } else {
        STACKPILOT_UNICODE
    };

    println!("{banner}");
}

#[cfg(test)]
mod tests {
    use super::{should_show_identity, STACKPILOT_ASCII, STACKPILOT_UNICODE};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn shows_identity_for_interactive_human_commands() {
        assert!(should_show_identity(&args(&["doctor"]), true, false));
        assert!(should_show_identity(&args(&["inspect", "."]), true, false));
    }

    #[test]
    fn suppresses_identity_for_automation_and_machine_safe_paths() {
        assert!(!should_show_identity(&args(&["doctor"]), false, false));
        assert!(!should_show_identity(&args(&["doctor"]), true, true));
        assert!(!should_show_identity(
            &args(&["new", "demo", "--non-interactive"]),
            true,
            false
        ));
        assert!(!should_show_identity(&args(&["--version"]), true, false));
        assert!(!should_show_identity(&args(&["--help"]), true, false));
    }

    #[test]
    fn generated_assets_match_the_shared_stackpilot_profile() {
        const DESCRIPTION: &str =
            "Opinionated project scaffolding for production-minded repositories";

        assert!(STACKPILOT_UNICODE.contains(DESCRIPTION));
        assert!(STACKPILOT_UNICODE.contains(['█', '▀', '▄']));
        assert!(STACKPILOT_ASCII.contains(DESCRIPTION));
        assert!(STACKPILOT_ASCII.contains('#'));
        assert!(!STACKPILOT_ASCII.contains(['█', '▀', '▄']));
    }
}
