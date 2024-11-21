#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum EnvVar {
    GHToken,
    GitHubToken,
    GHEnterpriseToken,
    GitHubEnterpriseToken,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Token {
    Env { value: String, var: EnvVar },
    Config { value: String, path: String },
    Keyring { value: String },
}

impl From<EnvToken> for Token {
    fn from(env_token: EnvToken) -> Self {
        Self::Env {
            value: env_token.value,
            var: env_token.var,
        }
    }
}

impl From<ConfigToken> for Token {
    fn from(config_token: ConfigToken) -> Self {
        Self::Config {
            value: config_token.value,
            path: config_token.path,
        }
    }
}

impl From<KeyringToken> for Token {
    fn from(keyring_token: KeyringToken) -> Self {
        Self::Keyring {
            value: keyring_token.value,
        }
    }
}

struct EnvToken {
    value: String,
    var: EnvVar,
}

struct ConfigToken {
    value: String,
    path: String,
}

struct KeyringToken {
    value: String,
}

pub fn token_for_host(host: &str) -> Option<Token> {
    token_from_env(host)
        .map(Token::from)
        .or_else(|| token_from_config(host).map(Token::from))
        .or_else(|| token_from_keyring(host).map(Token::from))
}

fn token_from_env(host: &str) -> Option<EnvToken> {
    // First we load the tokens that might be in the environment
    struct EnvTokens {
        gh_token: Option<EnvToken>,
        github_token: Option<EnvToken>,
        gh_enterprise_token: Option<EnvToken>,
        github_enterprise_token: Option<EnvToken>,
    }

    fn to_env_token(var: EnvVar) -> impl Fn(String) -> EnvToken {
        move |value| EnvToken { value, var }
    }

    // TODO: consider whether we should return an error here.
    let env_tokens = EnvTokens {
        gh_token: std::env::var("GH_TOKEN")
            .ok()
            .map(to_env_token(EnvVar::GHToken)),
        github_token: std::env::var("GITHUB_TOKEN")
            .ok()
            .map(to_env_token(EnvVar::GitHubToken)),
        gh_enterprise_token: std::env::var("GH_ENTERPRISE_TOKEN")
            .ok()
            .map(to_env_token(EnvVar::GHEnterpriseToken)),
        github_enterprise_token: std::env::var("GITHUB_ENTERPRISE_TOKEN")
            .ok()
            .map(to_env_token(EnvVar::GitHubEnterpriseToken)),
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

// TODO
fn token_from_keyring(_: &str) -> Option<KeyringToken> {
    None
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
                Some(Token::Env {
                    value: "gh-token-value".to_owned(),
                    var: EnvVar::GHToken
                }),
            )
        });
    }

    #[test]
    fn token_for_host_uses_github_token_variable_for_github_com() {
        temp_env::with_var("GITHUB_TOKEN", Some("github-token-value"), || {
            assert_eq!(
                token_for_host("github.com"),
                Some(Token::Env {
                    value: "github-token-value".to_owned(),
                    var: EnvVar::GitHubToken
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
                    Some(Token::Env {
                        value: "gh-token-value".to_owned(),
                        var: EnvVar::GHToken
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
                    Some(Token::Env {
                        value: "gh-enterprise-token-value".to_owned(),
                        var: EnvVar::GHEnterpriseToken
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
                    Some(Token::Env {
                        value: "github-enterprise-token-value".to_owned(),
                        var: EnvVar::GitHubEnterpriseToken
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
                    Some(Token::Env {
                        value: "gh-enterprise-token-value".to_owned(),
                        var: EnvVar::GHEnterpriseToken
                    })
                )
            },
        );
    }
}
