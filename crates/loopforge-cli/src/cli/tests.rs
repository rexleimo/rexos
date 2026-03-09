use clap::Parser;

use super::*;

#[test]
fn cli_primary_name_is_loopforge() {
    use clap::CommandFactory;
    assert_eq!(Cli::command().get_name(), "loopforge");
}

#[test]
fn cli_about_uses_loopforge_only_branding() {
    use clap::CommandFactory;
    let about = Cli::command()
        .get_about()
        .map(|s| s.to_string())
        .unwrap_or_default();
    assert!(
        about.contains("LoopForge"),
        "expected LoopForge about text, got: {about}"
    );
    assert!(
        !about.contains("RexOS"),
        "expected no RexOS mention, got: {about}"
    );
}

#[test]
fn cli_parses_config_validate_with_loopforge_binary_name() {
    let parsed = Cli::try_parse_from(["loopforge", "config", "validate"]);
    assert!(
        parsed.is_ok(),
        "expected `loopforge config validate` to parse, got: {parsed:?}"
    );
}

#[test]
fn cli_parses_release_check_subcommand() {
    let parsed = Cli::try_parse_from(["loopforge", "release", "check", "--tag", "v0.1.0"]);
    assert!(
        parsed.is_ok(),
        "expected `loopforge release check` to parse, got: {parsed:?}"
    );
}

#[test]
fn cli_parses_acp_events_subcommand() {
    let parsed = Cli::try_parse_from([
        "loopforge",
        "acp",
        "events",
        "--session",
        "s-1",
        "--limit",
        "20",
    ]);
    assert!(
        parsed.is_ok(),
        "expected `loopforge acp events` to parse, got: {parsed:?}"
    );
}

#[test]
fn cli_parses_acp_checkpoints_subcommand() {
    let parsed = Cli::try_parse_from(["loopforge", "acp", "checkpoints", "--session", "s-1"]);
    assert!(
        parsed.is_ok(),
        "expected `loopforge acp checkpoints` to parse, got: {parsed:?}"
    );
}

#[test]
fn cli_parses_agent_run_allowed_tools() {
    let parsed = Cli::try_parse_from([
        "loopforge",
        "agent",
        "run",
        "--workspace",
        ".",
        "--prompt",
        "x",
        "--allowed-tools",
        "fs_read,web_fetch",
    ]);
    assert!(
        parsed.is_ok(),
        "expected agent run with --allowed-tools to parse, got: {parsed:?}"
    );
}

#[test]
fn cli_parses_skills_list_subcommand() {
    let parsed = Cli::try_parse_from(["loopforge", "skills", "list", "--workspace", "."]);
    assert!(
        parsed.is_ok(),
        "expected `loopforge skills list` to parse, got: {parsed:?}"
    );
}

#[test]
fn cli_parses_skills_run_subcommand() {
    let parsed = Cli::try_parse_from([
        "loopforge",
        "skills",
        "run",
        "hello-skill",
        "--workspace",
        ".",
        "--input",
        "do x",
    ]);
    assert!(
        parsed.is_ok(),
        "expected `loopforge skills run` to parse, got: {parsed:?}"
    );
}

#[test]
fn cli_parses_onboard_subcommand() {
    let parsed = Cli::try_parse_from([
        "loopforge",
        "onboard",
        "--workspace",
        "loopforge-onboard-demo",
    ]);
    assert!(
        parsed.is_ok(),
        "expected `loopforge onboard` to parse, got: {parsed:?}"
    );
}

#[test]
fn cli_parses_onboard_starter_profile() {
    let parsed = Cli::try_parse_from([
        "loopforge",
        "onboard",
        "--workspace",
        "loopforge-onboard-demo",
        "--starter",
        "workspace-brief",
    ]);
    assert!(
        parsed.is_ok(),
        "expected `loopforge onboard --starter workspace-brief` to parse, got: {parsed:?}"
    );
}

#[test]
fn cli_parses_cron_tick_subcommand() {
    let parsed = Cli::try_parse_from(["loopforge", "cron", "tick"]);
    assert!(
        parsed.is_ok(),
        "expected `loopforge cron tick` to parse, got: {parsed:?}"
    );
}

#[test]
fn cli_parses_cron_worker_subcommand() {
    let parsed = Cli::try_parse_from(["loopforge", "cron", "worker"]);
    assert!(
        parsed.is_ok(),
        "expected `loopforge cron worker` to parse, got: {parsed:?}"
    );
}
