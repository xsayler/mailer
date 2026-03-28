use std::fmt;

#[derive(Debug)]
pub enum MailError {
    Network(String),
    Auth(String),
    Parse(String),
    Imap(String),
    Smtp(String),
}

impl fmt::Display for MailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MailError::Network(msg) => write!(f, "{msg}"),
            MailError::Auth(msg) => write!(f, "{msg}"),
            MailError::Parse(msg) => write!(f, "{msg}"),
            MailError::Imap(msg) => write!(f, "{msg}"),
            MailError::Smtp(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for MailError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_network_error() {
        let e = MailError::Network("connection refused".to_string());
        assert_eq!(e.to_string(), "connection refused");
    }

    #[test]
    fn display_auth_error() {
        let e = MailError::Auth("invalid credentials".to_string());
        assert_eq!(e.to_string(), "invalid credentials");
    }

    #[test]
    fn display_imap_error() {
        let e = MailError::Imap("SELECT failed".to_string());
        assert_eq!(e.to_string(), "SELECT failed");
    }

    #[test]
    fn display_smtp_error() {
        let e = MailError::Smtp("relay denied".to_string());
        assert_eq!(e.to_string(), "relay denied");
    }
}
