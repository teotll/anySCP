use async_trait::async_trait;
use russh::client;
use russh_keys::key::PublicKey;
#[cfg(test)]
use std::path::Path;

/// Handles server events for a single SSH connection.
pub struct SshClientHandler {
    host: String,
    port: u16,
}

impl SshClientHandler {
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }
}

#[cfg(test)]
fn verify_known_host_path<P: AsRef<Path>>(
    host: &str,
    port: u16,
    server_public_key: &PublicKey,
    known_hosts_path: P,
) -> Result<bool, russh_keys::Error> {
    russh_keys::known_hosts::check_known_hosts_path(
        host,
        port,
        server_public_key,
        known_hosts_path,
    )
}

fn verify_known_host(
    host: &str,
    port: u16,
    server_public_key: &PublicKey,
) -> Result<bool, russh_keys::Error> {
    russh_keys::known_hosts::check_known_hosts(host, port, server_public_key)
}

#[async_trait]
impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    /// Called when the server presents its host key.
    /// Accept only host keys already trusted in the user's known_hosts file.
    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        verify_known_host(&self.host, self.port, server_public_key).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use russh_keys::parse_public_key_base64;

    use super::verify_known_host_path;

    const HOST_KEY: &str =
        "AAAAC3NzaC1lZDI1NTE5AAAAIJdD7y3aLq454yWBdwLWbieU1ebz9/cu7/QEXn9OIeZJ";
    const OTHER_HOST_KEY: &str =
        "AAAAC3NzaC1lZDI1NTE5AAAAIA6rWI3G1sz07DnfFlrouTcysQlj2P+jpNSOEWD9OJ3X";

    #[test]
    fn verify_known_host_path_accepts_matching_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let known_hosts = dir.path().join("known_hosts");
        fs::write(
            &known_hosts,
            format!("[example.com]:2222 ssh-ed25519 {HOST_KEY}\n"),
        )
        .expect("write known_hosts");

        let key = parse_public_key_base64(HOST_KEY).expect("parse host key");

        assert!(verify_known_host_path("example.com", 2222, &key, &known_hosts).unwrap());
    }

    #[test]
    fn verify_known_host_path_rejects_missing_host() {
        let dir = tempfile::tempdir().expect("tempdir");
        let known_hosts = dir.path().join("known_hosts");
        fs::write(&known_hosts, format!("other.example ssh-ed25519 {HOST_KEY}\n"))
            .expect("write known_hosts");

        let key = parse_public_key_base64(HOST_KEY).expect("parse host key");

        assert!(!verify_known_host_path("example.com", 22, &key, &known_hosts).unwrap());
    }

    #[test]
    fn verify_known_host_path_errors_on_changed_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let known_hosts = dir.path().join("known_hosts");
        fs::write(
            &known_hosts,
            format!("example.com ssh-ed25519 {OTHER_HOST_KEY}\n"),
        )
        .expect("write known_hosts");

        let key = parse_public_key_base64(HOST_KEY).expect("parse host key");

        assert!(verify_known_host_path("example.com", 22, &key, &known_hosts).is_err());
    }
}
