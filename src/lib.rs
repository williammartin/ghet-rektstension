use core::str;
use std::{process::Command, str::Utf8Error};

#[derive(Debug, PartialEq, Eq)]
pub enum Source {
    Env(Var),
    Config(String), // path to file
    Keyring,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Var {
    GHToken,
    GitHubToken,
    GHEnterpriseToken,
    GitHubEnterpriseToken,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    pub value: String,
    pub source: Source,
}

impl From<EnvToken> for Token {
    fn from(env_token: EnvToken) -> Self {
        Self {
            value: env_token.value,
            source: Source::Env(env_token.var),
        }
    }
}

impl From<ConfigToken> for Token {
    fn from(config_token: ConfigToken) -> Self {
        Self {
            value: config_token.value,
            source: Source::Config(config_token.path),
        }
    }
}

impl From<KeyringToken> for Token {
    fn from(keyring_token: KeyringToken) -> Self {
        Self {
            value: keyring_token.value,
            source: Source::Keyring,
        }
    }
}

struct EnvToken {
    value: String,
    var: Var,
}

struct ConfigToken {
    value: String,
    path: String,
}

#[derive(Debug, PartialEq, Eq)]
struct KeyringToken {
    value: String,
}

pub fn token_for_host(host: &str) -> Option<Token> {
    token_from_env(host)
        .map(Token::from)
        .or_else(|| token_from_config(host).map(Token::from))
        .or_else(|| {
            token_from_keyring(host)
                .map_err(|err| panic!("{err:?}"))
                .ok()
                .flatten()
                .map(Token::from)
        })
}

fn token_from_env(host: &str) -> Option<EnvToken> {
    // First we load the tokens that might be in the environment
    struct EnvTokens {
        gh_token: Option<EnvToken>,
        github_token: Option<EnvToken>,
        gh_enterprise_token: Option<EnvToken>,
        github_enterprise_token: Option<EnvToken>,
    }

    fn to_env_token(var: Var) -> impl Fn(String) -> EnvToken {
        move |value| EnvToken { value, var }
    }

    // TODO: consider whether we should return an error here.
    let env_tokens = EnvTokens {
        gh_token: std::env::var("GH_TOKEN")
            .ok()
            .map(to_env_token(Var::GHToken)),
        github_token: std::env::var("GITHUB_TOKEN")
            .ok()
            .map(to_env_token(Var::GitHubToken)),
        gh_enterprise_token: std::env::var("GH_ENTERPRISE_TOKEN")
            .ok()
            .map(to_env_token(Var::GHEnterpriseToken)),
        github_enterprise_token: std::env::var("GITHUB_ENTERPRISE_TOKEN")
            .ok()
            .map(to_env_token(Var::GitHubEnterpriseToken)),
    };

    match host {
        // TODO: do GHEC and localhost
        "github.com" => env_tokens.gh_token.or(env_tokens.github_token),
        _ => env_tokens
            .gh_enterprise_token
            .or(env_tokens.github_enterprise_token),
    }
}

// TODO
fn token_from_config(_: &str) -> Option<ConfigToken> {
    None
}

#[derive(Debug, PartialEq, Eq)]
pub enum TokenFromKeyringError {
    FailToExecute(std::io::ErrorKind),
    StdoutNotUTF8(Utf8Error),
    StdErrorNotUTF8(Utf8Error),
    OutputStatusFail(String),
}

fn token_from_keyring(host: &str) -> Result<Option<KeyringToken>, TokenFromKeyringError> {
    let args;

    #[cfg(test)]
    {
        args = ["auth", "token", "--hostname", host];
    }

    #[cfg(not(test))]
    {
        args = ["auth", "token", "--secure-storage", "--hostname", host];
    }

    Command::new("gh")
        .args(args)
        .output()
        .map_err(|err| TokenFromKeyringError::FailToExecute(err.kind()))
        .and_then(|output| {
            if output.status.success() {
                str::from_utf8(&output.stdout)
                    .map_err(TokenFromKeyringError::StdoutNotUTF8)
                    .map(|value| {
                        Some(KeyringToken {
                            value: value.trim().to_string(),
                        })
                    })
            } else {
                str::from_utf8(&output.stderr)
                    .map_err(TokenFromKeyringError::StdErrorNotUTF8)
                    .and_then(|error_string| {
                        if error_string.contains("no oauth token found") {
                            Ok(None)
                        } else {
                            Err(TokenFromKeyringError::OutputStatusFail(
                                error_string.to_string(),
                            ))
                        }
                    })
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_for_host_returns_none_when_no_match() {
        assert_eq!(token_for_host("unknown-host.com"), None)
    }

    #[test]
    fn token_for_host_uses_gh_token_variable_for_github_com() {
        temp_env::with_var("GH_TOKEN", Some("gh-token-value"), || {
            assert_eq!(
                token_for_host("github.com"),
                Some(Token {
                    value: "gh-token-value".to_owned(),
                    source: Source::Env(Var::GHToken)
                }),
            )
        });
    }

    #[test]
    fn token_for_host_uses_github_token_variable_for_github_com() {
        temp_env::with_var("GITHUB_TOKEN", Some("github-token-value"), || {
            assert_eq!(
                token_for_host("github.com"),
                Some(Token {
                    value: "github-token-value".to_owned(),
                    source: Source::Env(Var::GitHubToken)
                })
            )
        });
    }

    #[test]
    fn token_for_host_uses_gh_over_github_token_variable_for_github_com() {
        temp_env::with_vars(
            [
                ("GH_TOKEN", Some("gh-token-value")),
                ("GITHUB_TOKEN", Some("github-token-value")),
            ],
            || {
                assert_eq!(
                    token_for_host("github.com"),
                    Some(Token {
                        value: "gh-token-value".to_owned(),
                        source: Source::Env(Var::GHToken)
                    })
                )
            },
        );
    }

    #[test]
    fn token_for_host_uses_gh_enterprise_token_for_any_other_hosts() {
        temp_env::with_var(
            "GH_ENTERPRISE_TOKEN",
            Some("gh-enterprise-token-value"),
            || {
                assert_eq!(
                    token_for_host("my.ghes.com"),
                    Some(Token {
                        value: "gh-enterprise-token-value".to_owned(),
                        source: Source::Env(Var::GHEnterpriseToken)
                    })
                )
            },
        );
    }

    #[test]
    fn token_for_host_uses_github_enterprise_token_for_any_other_hosts() {
        temp_env::with_var(
            "GITHUB_ENTERPRISE_TOKEN",
            Some("github-enterprise-token-value"),
            || {
                assert_eq!(
                    token_for_host("my.ghes.com"),
                    Some(Token {
                        value: "github-enterprise-token-value".to_owned(),
                        source: Source::Env(Var::GitHubEnterpriseToken)
                    })
                )
            },
        );
    }

    #[test]
    fn token_for_host_uses_gh_over_github_token_variable_for_other_hosts() {
        temp_env::with_vars(
            [
                ("GH_ENTERPRISE_TOKEN", Some("gh-enterprise-token-value")),
                (
                    "GITHUB_ENTERPRISE_TOKEN",
                    Some("github-enterprise-token-value"),
                ),
            ],
            || {
                assert_eq!(
                    token_for_host("my.ghes.com"),
                    Some(Token {
                        value: "gh-enterprise-token-value".to_owned(),
                        source: Source::Env(Var::GHEnterpriseToken)
                    })
                )
            },
        );
    }

    #[test]
    fn token_for_keyring_asks_for_token_from_gh() {
        temp_env::with_var("GH_TOKEN", Some("gh-token-value"), || {
            assert_eq!(
                token_from_keyring("github.com"),
                Ok(Some(KeyringToken {
                    value: "gh-token-value".to_owned(),
                })),
            )
        });
    }
}
