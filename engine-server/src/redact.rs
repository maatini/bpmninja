//! Maskierung von Zugangsdaten in Verbindungs-URLs.
//!
//! NATS-URLs können Credentials (`nats://user:pass@host:4222`) oder einen Token
//! im Query-String (`?token=…`) enthalten. Alles, was geloggt oder über die API
//! ausgeliefert wird, läuft durch [`redact_nats_url`].

/// Entfernt Zugangsdaten aus einer NATS-URL.
///
/// - Userinfo wird maskiert: `nats://user:pass@host:4222` → `nats://***@host:4222`
/// - Query-Strings (z. B. `?token=…`) werden komplett maskiert: `?***`
/// - Kommagetrennte Serverlisten werden je Eintrag behandelt
/// - URLs ohne Zugangsdaten bleiben unverändert
///
/// Bewusst ohne `url`-Crate: NATS erlaubt auch Serverlisten und einzelne
/// `host:port`-Einträge ohne Schema, die ein strikter Parser ablehnen würde.
/// Die Funktion ist idempotent — bereits maskierte URLs bleiben unverändert.
pub fn redact_nats_url(raw: &str) -> String {
    raw.split(',')
        .map(redact_single_url)
        .collect::<Vec<_>>()
        .join(",")
}

fn redact_single_url(entry: &str) -> String {
    let entry = entry.trim();
    if entry.is_empty() {
        return String::new();
    }

    let (without_query, has_query) = match entry.split_once('?') {
        Some((base, _)) => (base, true),
        None => (entry, false),
    };
    let (scheme, rest) = match without_query.split_once("://") {
        Some((scheme, rest)) => (Some(scheme), rest),
        None => (None, without_query),
    };
    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, Some(path)),
        None => (rest, None),
    };
    // Das letzte '@' trennt die Userinfo, da '@' in Passwörtern kodiert sein muss.
    let authority = match authority.rsplit_once('@') {
        Some((_, host)) => format!("***@{host}"),
        None => authority.to_string(),
    };

    let mut out = String::with_capacity(entry.len());
    if let Some(scheme) = scheme {
        out.push_str(scheme);
        out.push_str("://");
    }
    out.push_str(&authority);
    if let Some(path) = path {
        out.push('/');
        out.push_str(path);
    }
    if has_query {
        out.push_str("?***");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::redact_nats_url;

    #[test]
    fn redacts_user_and_password() {
        let redacted = redact_nats_url("nats://bpmninja:s3cret@nats:4222");
        assert_eq!(redacted, "nats://***@nats:4222");
        assert!(!redacted.contains("bpmninja"));
        assert!(!redacted.contains("s3cret"));
    }

    #[test]
    fn redacts_token_query() {
        let redacted = redact_nats_url("nats://nats:4222?token=abc123");
        assert_eq!(redacted, "nats://nats:4222?***");
        assert!(!redacted.contains("abc123"));
    }

    #[test]
    fn redacts_userinfo_and_query_together() {
        let redacted = redact_nats_url("nats://alice:pw@nats:4222/?token=abc123");
        assert_eq!(redacted, "nats://***@nats:4222/?***");
    }

    #[test]
    fn redacts_token_only_userinfo() {
        let redacted = redact_nats_url("nats://t0k3n@nats:4222");
        assert_eq!(redacted, "nats://***@nats:4222");
    }

    #[test]
    fn keeps_url_without_credentials() {
        assert_eq!(
            redact_nats_url("nats://localhost:4222"),
            "nats://localhost:4222"
        );
        assert_eq!(redact_nats_url("nats://nats:4222"), "nats://nats:4222");
        assert_eq!(
            redact_nats_url("tls://nats.internal:4222"),
            "tls://nats.internal:4222"
        );
    }

    #[test]
    fn redacts_each_entry_of_server_list() {
        let redacted = redact_nats_url("nats://u:p@a:4222, nats://b:4222");
        assert_eq!(redacted, "nats://***@a:4222,nats://b:4222");
        assert!(!redacted.contains(":p@"));
    }

    #[test]
    fn redacts_percent_encoded_at_in_password() {
        let redacted = redact_nats_url("nats://user:p%40ss@host:4222");
        assert_eq!(redacted, "nats://***@host:4222");
    }

    #[test]
    fn is_idempotent() {
        let once = redact_nats_url("nats://u:p@host:4222?token=abc123");
        assert_eq!(redact_nats_url(&once), once);
    }
}
