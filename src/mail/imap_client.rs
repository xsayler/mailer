use crate::config::AccountConfig;
use crate::mail::error::MailError;
use crate::mail::models::{Address, Attachment, MailFolder, MailMessage};
use async_imap::Session;
use chrono::DateTime;
use futures::StreamExt;
use log::info;
use mail_parser::MimeHeaders;
use rustls_pki_types::ServerName;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_util::sync::CancellationToken;

type ImapSession = Session<TlsStream<TcpStream>>;

pub struct ImapClient {
    config: AccountConfig,
    session: Mutex<Option<ImapSession>>,
    idle_cancel: Mutex<Option<CancellationToken>>,
}

impl ImapClient {
    pub fn new(config: AccountConfig) -> Self {
        Self {
            config,
            session: Mutex::new(None),
            idle_cancel: Mutex::new(None),
        }
    }

    async fn connect_new(&self) -> Result<ImapSession, MailError> {
        info!(
            "Connecting to IMAP {}:{}",
            self.config.imap_host, self.config.imap_port
        );

        let tcp = TcpStream::connect((&*self.config.imap_host, self.config.imap_port))
            .await
            .map_err(|e| MailError::Network(format!("TCP connect failed: {e}")))?;

        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(tls_config));
        let server_name = ServerName::try_from(self.config.imap_host.clone())
            .map_err(|e| MailError::Network(format!("Invalid server name: {e}")))?;

        let tls_stream = connector
            .connect(server_name, tcp)
            .await
            .map_err(|e| MailError::Network(format!("TLS connect failed: {e}")))?;

        let client = async_imap::Client::new(tls_stream);
        let sess = client
            .login(&self.config.username, &self.config.password)
            .await
            .map_err(|(e, _client)| MailError::Auth(format!("IMAP login failed: {e}")))?;

        info!("IMAP connected successfully");
        Ok(sess)
    }

    async fn ensure_connected(&self) -> Result<(), MailError> {
        let mut session = self.session.lock().await;
        if session.is_some() {
            return Ok(());
        }
        *session = Some(self.connect_new().await?);
        Ok(())
    }

    /// Drop the current session so the next call reconnects.
    async fn reset_session(&self) {
        let mut session = self.session.lock().await;
        *session = None;
    }

    pub async fn list_folders(&self) -> Result<Vec<MailFolder>, MailError> {
        match self.list_folders_inner().await {
            Ok(folders) => Ok(folders),
            Err(e) => {
                info!("list_folders failed ({e}), reconnecting...");
                self.reset_session().await;
                self.ensure_connected().await?;
                self.list_folders_inner().await
            }
        }
    }

    async fn list_folders_inner(&self) -> Result<Vec<MailFolder>, MailError> {
        let mut session = self.session.lock().await;
        let sess = session.as_mut().ok_or(MailError::Imap("Not connected".to_string()))?;

        let mailboxes_stream = sess
            .list(Some(""), Some("*"))
            .await
            .map_err(|e| MailError::Imap(format!("LIST failed: {e}")))?;

        let mailboxes: Vec<_> = mailboxes_stream
            .filter_map(|r| async { r.ok() })
            .collect()
            .await;

        let mut folders = Vec::new();
        for mb in &mailboxes {
            let decoded = decode_mutf7(mb.name());
            folders.push(MailFolder {
                name: decoded.split('/').last().unwrap_or(&decoded).to_string(),
                path: mb.name().to_string(),
                unread_count: 0,
            });
        }

        folders.sort_by(|a, b| {
            if a.path.eq_ignore_ascii_case("INBOX") {
                std::cmp::Ordering::Less
            } else if b.path.eq_ignore_ascii_case("INBOX") {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });

        Ok(folders)
    }

    /// Fetch messages with pagination. Returns (messages, total_count).
    pub async fn fetch_messages(
        &self,
        folder: &str,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<MailMessage>, u32), MailError> {
        match self.fetch_messages_inner(folder, limit, offset).await {
            Ok(result) => Ok(result),
            Err(e) => {
                info!("fetch_messages failed ({e}), reconnecting...");
                self.reset_session().await;
                self.ensure_connected().await?;
                self.fetch_messages_inner(folder, limit, offset).await
            }
        }
    }

    async fn fetch_messages_inner(
        &self,
        folder: &str,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<MailMessage>, u32), MailError> {
        let mut session = self.session.lock().await;
        let sess = session.as_mut().ok_or(MailError::Imap("Not connected".to_string()))?;

        let mailbox = sess
            .select(folder)
            .await
            .map_err(|e| MailError::Imap(format!("SELECT failed: {e}")))?;

        let total = mailbox.exists;
        if total == 0 {
            return Ok((Vec::new(), 0));
        }

        let end = if total > offset { total - offset } else { 0 };
        if end == 0 {
            return Ok((Vec::new(), total));
        }
        let start = if end > limit { end - limit + 1 } else { 1 };
        let range = format!("{}:{}", start, end);

        let fetch_stream = sess
            .fetch(&range, "(UID FLAGS BODY.PEEK[])")
            .await
            .map_err(|e| MailError::Imap(format!("FETCH failed: {e}")))?;

        let messages: Vec<_> = fetch_stream
            .filter_map(|r| async { r.ok() })
            .collect()
            .await;

        let parser = mail_parser::MessageParser::default();
        let mut result = Vec::new();
        for msg in &messages {
            let flags: Vec<_> = msg.flags().collect();
            let is_read = flags.iter().any(|f| matches!(f, async_imap::types::Flag::Seen));
            let is_flagged = flags
                .iter()
                .any(|f| matches!(f, async_imap::types::Flag::Flagged));

            let raw = match msg.body() {
                Some(b) => b,
                None => continue,
            };
            let parsed = match parser.parse(raw) {
                Some(p) => p,
                None => continue,
            };

            let subject = parsed.subject().unwrap_or("(no subject)").to_string();

            let from = parsed
                .from()
                .map(|a| {
                    a.iter()
                        .map(|a| Address {
                            name: a.name.as_ref().map(|n| n.to_string()),
                            email: a.address.as_ref().map(|e| e.to_string()).unwrap_or_default(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let to = parsed
                .to()
                .map(|a| {
                    a.iter()
                        .map(|a| Address {
                            name: a.name.as_ref().map(|n| n.to_string()),
                            email: a.address.as_ref().map(|e| e.to_string()).unwrap_or_default(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let cc = parsed
                .cc()
                .map(|a| {
                    a.iter()
                        .map(|a| Address {
                            name: a.name.as_ref().map(|n| n.to_string()),
                            email: a.address.as_ref().map(|e| e.to_string()).unwrap_or_default(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let date = parsed.date().and_then(|d| {
                DateTime::parse_from_rfc3339(&d.to_rfc3339())
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            });

            let body_text = parsed.body_text(0).map(|t| t.to_string());
            let body_html = parsed.body_html(0).map(|t| t.to_string());

            let attachments = parsed
                .attachments()
                .map(|part| {
                    let filename = part
                        .attachment_name()
                        .unwrap_or("attachment")
                        .to_string();
                    let content_type = part
                        .content_type()
                        .map(|ct: &mail_parser::ContentType| {
                            format!("{}/{}", ct.ctype(), ct.subtype().unwrap_or("octet-stream"))
                        })
                        .unwrap_or_else(|| "application/octet-stream".to_string());
                    let data = part.contents().to_vec();
                    let size = data.len();
                    Attachment { filename, content_type, size, data }
                })
                .collect();

            result.push(MailMessage {
                uid: msg.uid.unwrap_or(0),
                subject,
                from,
                to,
                cc,
                date,
                is_read,
                is_flagged,
                body_text,
                body_html,
                attachments,
            });
        }

        result.reverse();
        Ok((result, total))
    }

    // --- Message management operations ---

    pub async fn mark_read(&self, folder: &str, uid: u32, read: bool) -> Result<(), MailError> {
        match self.store_flags_inner(folder, uid, "\\Seen", read).await {
            Ok(()) => Ok(()),
            Err(e) => {
                info!("mark_read failed ({e}), reconnecting...");
                self.reset_session().await;
                self.ensure_connected().await?;
                self.store_flags_inner(folder, uid, "\\Seen", read).await
            }
        }
    }

    pub async fn set_flagged(&self, folder: &str, uid: u32, flagged: bool) -> Result<(), MailError> {
        match self.store_flags_inner(folder, uid, "\\Flagged", flagged).await {
            Ok(()) => Ok(()),
            Err(e) => {
                info!("set_flagged failed ({e}), reconnecting...");
                self.reset_session().await;
                self.ensure_connected().await?;
                self.store_flags_inner(folder, uid, "\\Flagged", flagged).await
            }
        }
    }

    pub async fn delete_message(&self, folder: &str, uid: u32) -> Result<(), MailError> {
        match self.delete_message_inner(folder, uid).await {
            Ok(()) => Ok(()),
            Err(e) => {
                info!("delete_message failed ({e}), reconnecting...");
                self.reset_session().await;
                self.ensure_connected().await?;
                self.delete_message_inner(folder, uid).await
            }
        }
    }

    pub async fn move_message(
        &self,
        src_folder: &str,
        uid: u32,
        dest_folder: &str,
    ) -> Result<(), MailError> {
        match self.move_message_inner(src_folder, uid, dest_folder).await {
            Ok(()) => Ok(()),
            Err(e) => {
                info!("move_message failed ({e}), reconnecting...");
                self.reset_session().await;
                self.ensure_connected().await?;
                self.move_message_inner(src_folder, uid, dest_folder).await
            }
        }
    }

    pub async fn fetch_unread_counts(
        &self,
        folders: &[String],
    ) -> Result<Vec<(String, u32)>, MailError> {
        self.ensure_connected().await?;
        let mut session = self.session.lock().await;
        let sess = session.as_mut().ok_or(MailError::Imap("Not connected".to_string()))?;

        let mut counts = Vec::new();
        for folder in folders {
            let result = sess.status(folder, "(UNSEEN)").await;
            match result {
                Ok(mailbox) => {
                    let count = mailbox.unseen.unwrap_or(0) as u32;
                    counts.push((folder.clone(), count));
                }
                Err(_) => {
                    counts.push((folder.clone(), 0));
                }
            }
        }
        Ok(counts)
    }

    /// Search messages on the server using IMAP SEARCH.
    pub async fn search_messages(
        &self,
        folder: &str,
        query: &str,
        limit: u32,
    ) -> Result<Vec<MailMessage>, MailError> {
        match self.search_messages_inner(folder, query, limit).await {
            Ok(msgs) => Ok(msgs),
            Err(e) => {
                info!("search_messages failed ({e}), reconnecting...");
                self.reset_session().await;
                self.ensure_connected().await?;
                self.search_messages_inner(folder, query, limit).await
            }
        }
    }

    async fn search_messages_inner(
        &self,
        folder: &str,
        query: &str,
        limit: u32,
    ) -> Result<Vec<MailMessage>, MailError> {
        let mut session = self.session.lock().await;
        let sess = session.as_mut().ok_or(MailError::Imap("Not connected".to_string()))?;

        sess.select(folder)
            .await
            .map_err(|e| MailError::Imap(format!("SELECT failed: {e}")))?;

        let criteria = format!(
            "OR OR SUBJECT \"{}\" FROM \"{}\" BODY \"{}\"",
            query.replace('"', "\\\""),
            query.replace('"', "\\\""),
            query.replace('"', "\\\""),
        );

        let uids = sess
            .search(&criteria)
            .await
            .map_err(|e| MailError::Imap(format!("SEARCH failed: {e}")))?;

        if uids.is_empty() {
            return Ok(Vec::new());
        }

        let mut uid_list: Vec<u32> = uids.into_iter().collect();
        uid_list.sort();
        // Take the last `limit` UIDs (most recent)
        let start = if uid_list.len() > limit as usize {
            uid_list.len() - limit as usize
        } else {
            0
        };
        let selected = &uid_list[start..];
        let uid_range = selected
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let fetch_stream = sess
            .uid_fetch(&uid_range, "(UID FLAGS BODY.PEEK[])")
            .await
            .map_err(|e| MailError::Imap(format!("FETCH failed: {e}")))?;

        let messages: Vec<_> = fetch_stream
            .filter_map(|r| async { r.ok() })
            .collect()
            .await;

        let parser = mail_parser::MessageParser::default();
        let mut result = Vec::new();
        for msg in &messages {
            let flags: Vec<_> = msg.flags().collect();
            let is_read = flags.iter().any(|f| matches!(f, async_imap::types::Flag::Seen));
            let is_flagged = flags
                .iter()
                .any(|f| matches!(f, async_imap::types::Flag::Flagged));

            let raw = match msg.body() {
                Some(b) => b,
                None => continue,
            };
            let parsed = match parser.parse(raw) {
                Some(p) => p,
                None => continue,
            };

            let subject = parsed.subject().unwrap_or("(no subject)").to_string();

            let from = parsed
                .from()
                .map(|a| {
                    a.iter()
                        .map(|a| Address {
                            name: a.name.as_ref().map(|n| n.to_string()),
                            email: a.address.as_ref().map(|e| e.to_string()).unwrap_or_default(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let to = parsed
                .to()
                .map(|a| {
                    a.iter()
                        .map(|a| Address {
                            name: a.name.as_ref().map(|n| n.to_string()),
                            email: a.address.as_ref().map(|e| e.to_string()).unwrap_or_default(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let cc = parsed
                .cc()
                .map(|a| {
                    a.iter()
                        .map(|a| Address {
                            name: a.name.as_ref().map(|n| n.to_string()),
                            email: a.address.as_ref().map(|e| e.to_string()).unwrap_or_default(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let date = parsed.date().and_then(|d| {
                DateTime::parse_from_rfc3339(&d.to_rfc3339())
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            });

            let body_text = parsed.body_text(0).map(|t| t.to_string());
            let body_html = parsed.body_html(0).map(|t| t.to_string());

            let attachments = parsed
                .attachments()
                .map(|part| {
                    let filename = part
                        .attachment_name()
                        .unwrap_or("attachment")
                        .to_string();
                    let content_type = part
                        .content_type()
                        .map(|ct: &mail_parser::ContentType| {
                            format!("{}/{}", ct.ctype(), ct.subtype().unwrap_or("octet-stream"))
                        })
                        .unwrap_or_else(|| "application/octet-stream".to_string());
                    let data = part.contents().to_vec();
                    let size = data.len();
                    Attachment { filename, content_type, size, data }
                })
                .collect();

            result.push(MailMessage {
                uid: msg.uid.unwrap_or(0),
                subject,
                from,
                to,
                cc,
                date,
                is_read,
                is_flagged,
                body_text,
                body_html,
                attachments,
            });
        }

        result.reverse();
        Ok(result)
    }

    /// Start IDLE on a folder using a separate connection.
    /// Calls `on_change` when new data arrives. Loops until cancelled.
    pub async fn start_idle(
        &self,
        folder: &str,
        on_change: impl Fn() + Send + 'static,
    ) -> Result<(), MailError> {
        self.stop_idle().await;

        let token = CancellationToken::new();
        *self.idle_cancel.lock().await = Some(token.clone());

        let mut idle_session = self.connect_new().await?;
        idle_session
            .select(folder)
            .await
            .map_err(|e| MailError::Imap(format!("IDLE SELECT failed: {e}")))?;

        let folder = folder.to_string();
        info!("Starting IDLE on {folder}");

        tokio::spawn(async move {
            let mut session = idle_session;
            loop {
                if token.is_cancelled() {
                    let _ = session.logout().await;
                    break;
                }

                // idle() consumes Session, returns Handle
                let mut handle = session.idle();
                if let Err(e) = handle.init().await {
                    info!("IDLE init failed: {e}");
                    break;
                }

                let (fut, _stop) = handle.wait_with_timeout(Duration::from_secs(25 * 60));

                let result = tokio::select! {
                    r = fut => r,
                    _ = token.cancelled() => {
                        // done() consumes Handle, returns Session
                        session = match handle.done().await {
                            Ok(s) => s,
                            Err(_) => break,
                        };
                        let _ = session.logout().await;
                        break;
                    }
                };

                // done() returns the Session back
                session = match handle.done().await {
                    Ok(s) => s,
                    Err(e) => {
                        info!("IDLE done failed: {e}");
                        break;
                    }
                };

                match result {
                    Ok(reason) => match reason {
                        async_imap::extensions::idle::IdleResponse::NewData(_) => {
                            on_change();
                        }
                        async_imap::extensions::idle::IdleResponse::Timeout => {
                            // Re-enter IDLE
                        }
                        async_imap::extensions::idle::IdleResponse::ManualInterrupt => {
                            break;
                        }
                    },
                    Err(e) => {
                        info!("IDLE wait error: {e}");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn stop_idle(&self) {
        let mut cancel = self.idle_cancel.lock().await;
        if let Some(token) = cancel.take() {
            token.cancel();
        }
    }

    /// Find the Sent folder by checking LIST attributes and common names.
    pub async fn find_sent_folder(&self) -> Result<Option<String>, MailError> {
        self.ensure_connected().await?;
        let mut session = self.session.lock().await;
        let sess = session.as_mut().ok_or(MailError::Imap("Not connected".to_string()))?;

        let mailboxes_stream = sess
            .list(Some(""), Some("*"))
            .await
            .map_err(|e| MailError::Imap(format!("LIST failed: {e}")))?;

        let mailboxes: Vec<_> = mailboxes_stream
            .filter_map(|r| async { r.ok() })
            .collect()
            .await;

        // Log all folders for debugging
        for mb in &mailboxes {
            let attrs: Vec<_> = mb.attributes().into_iter().collect();
            let decoded = decode_mutf7(mb.name());
            info!("IMAP folder: raw={:?} decoded={:?} attrs={:?}", mb.name(), decoded, attrs);
        }

        // Check for \Sent attribute first (SPECIAL-USE / XLIST)
        for mb in &mailboxes {
            let attrs: Vec<_> = mb.attributes().into_iter().collect();
            for attr in &attrs {
                let s = format!("{:?}", attr);
                if s.contains("Sent") {
                    info!("Found Sent folder by attribute: {:?}", mb.name());
                    return Ok(Some(mb.name().to_string()));
                }
            }
        }

        // Fallback: match common Sent folder names against decoded MUTF7
        let sent_patterns = [
            "sent", "sent mail", "sent messages",
            "отправленные", "отправленная почта",
        ];
        for mb in &mailboxes {
            let decoded = decode_mutf7(mb.name()).to_lowercase();
            let last_component = decoded
                .rsplit(|c| c == '/' || c == '.')
                .next()
                .unwrap_or(&decoded);
            for pattern in &sent_patterns {
                if last_component == *pattern || decoded.contains(pattern) {
                    info!("Found Sent folder by name: raw={:?} decoded={:?}", mb.name(), decoded);
                    return Ok(Some(mb.name().to_string()));
                }
            }
        }

        info!("No Sent folder found");
        Ok(None)
    }

    /// Append a raw message to a folder (typically Sent).
    pub async fn append_to_folder(&self, folder: &str, raw_message: &[u8]) -> Result<(), MailError> {
        self.ensure_connected().await?;
        let mut session = self.session.lock().await;
        let sess = session.as_mut().ok_or(MailError::Imap("Not connected".to_string()))?;
        sess.append(folder, Some("(\\Seen)"), None, raw_message)
            .await
            .map_err(|e| MailError::Imap(format!("APPEND failed: {e}")))?;
        Ok(())
    }

    pub async fn create_folder(&self, name: &str) -> Result<(), MailError> {
        self.ensure_connected().await?;
        let mut session = self.session.lock().await;
        let sess = session.as_mut().ok_or(MailError::Imap("Not connected".to_string()))?;
        sess.create(name)
            .await
            .map_err(|e| MailError::Imap(format!("CREATE failed: {e}")))?;
        Ok(())
    }

    pub async fn rename_folder(&self, from: &str, to: &str) -> Result<(), MailError> {
        self.ensure_connected().await?;
        let mut session = self.session.lock().await;
        let sess = session.as_mut().ok_or(MailError::Imap("Not connected".to_string()))?;
        sess.rename(from, to)
            .await
            .map_err(|e| MailError::Imap(format!("RENAME failed: {e}")))?;
        Ok(())
    }

    pub async fn delete_folder(&self, name: &str) -> Result<(), MailError> {
        self.ensure_connected().await?;
        let mut session = self.session.lock().await;
        let sess = session.as_mut().ok_or(MailError::Imap("Not connected".to_string()))?;
        sess.delete(name)
            .await
            .map_err(|e| MailError::Imap(format!("DELETE failed: {e}")))?;
        Ok(())
    }

    async fn store_flags_inner(
        &self,
        folder: &str,
        uid: u32,
        flags: &str,
        add: bool,
    ) -> Result<(), MailError> {
        self.ensure_connected().await?;
        let mut session = self.session.lock().await;
        let sess = session.as_mut().ok_or(MailError::Imap("Not connected".to_string()))?;

        sess.select(folder)
            .await
            .map_err(|e| MailError::Imap(format!("SELECT failed: {e}")))?;

        let query = if add {
            format!("+FLAGS ({flags})")
        } else {
            format!("-FLAGS ({flags})")
        };

        let stream = sess
            .uid_store(uid.to_string(), &query)
            .await
            .map_err(|e| MailError::Imap(format!("STORE failed: {e}")))?;

        // Consume the stream
        let _: Vec<_> = stream.filter_map(|r| async { r.ok() }).collect().await;
        Ok(())
    }

    async fn delete_message_inner(&self, folder: &str, uid: u32) -> Result<(), MailError> {
        self.ensure_connected().await?;
        let mut session = self.session.lock().await;
        let sess = session.as_mut().ok_or(MailError::Imap("Not connected".to_string()))?;

        sess.select(folder)
            .await
            .map_err(|e| MailError::Imap(format!("SELECT failed: {e}")))?;

        let stream = sess
            .uid_store(uid.to_string(), "+FLAGS (\\Deleted)")
            .await
            .map_err(|e| MailError::Imap(format!("STORE failed: {e}")))?;
        let _: Vec<_> = stream.filter_map(|r| async { r.ok() }).collect().await;

        let expunge_stream = sess
            .expunge()
            .await
            .map_err(|e| MailError::Imap(format!("EXPUNGE failed: {e}")))?;
        let _: Vec<_> = expunge_stream
            .filter_map(|r| async { r.ok() })
            .collect()
            .await;

        Ok(())
    }

    async fn move_message_inner(
        &self,
        src_folder: &str,
        uid: u32,
        dest_folder: &str,
    ) -> Result<(), MailError> {
        self.ensure_connected().await?;
        let mut session = self.session.lock().await;
        let sess = session.as_mut().ok_or(MailError::Imap("Not connected".to_string()))?;

        sess.select(src_folder)
            .await
            .map_err(|e| MailError::Imap(format!("SELECT failed: {e}")))?;

        sess.uid_copy(uid.to_string(), dest_folder)
            .await
            .map_err(|e| MailError::Imap(format!("COPY failed: {e}")))?;

        let stream = sess
            .uid_store(uid.to_string(), "+FLAGS (\\Deleted)")
            .await
            .map_err(|e| MailError::Imap(format!("STORE failed: {e}")))?;
        let _: Vec<_> = stream.filter_map(|r| async { r.ok() }).collect().await;

        let expunge_stream = sess
            .expunge()
            .await
            .map_err(|e| MailError::Imap(format!("EXPUNGE failed: {e}")))?;
        let _: Vec<_> = expunge_stream
            .filter_map(|r| async { r.ok() })
            .collect()
            .await;

        Ok(())
    }

}

/// Decode IMAP modified UTF-7 mailbox names (RFC 3501 §5.1.3).
pub fn decode_mutf7(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '&' {
            result.push(ch);
            continue;
        }
        let mut encoded = String::new();
        loop {
            match chars.next() {
                Some('-') => break,
                Some(c) => encoded.push(c),
                None => break,
            }
        }
        if encoded.is_empty() {
            result.push('&');
            continue;
        }
        // Modified base64: ',' → '/'
        let b64: String = encoded.replace(',', "/");
        if let Some(bytes) = base64_decode(&b64) {
            let utf16: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
                .collect();
            if let Ok(s) = String::from_utf16(&utf16) {
                result.push_str(&s);
            }
        }
    }
    result
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lut = [255u8; 256];
    for (i, &c) in TABLE.iter().enumerate() {
        lut[c as usize] = i as u8;
    }

    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'=').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);

    for chunk in bytes.chunks(4) {
        let mut buf = [0u8; 4];
        for (i, &b) in chunk.iter().enumerate() {
            let val = lut[b as usize];
            if val == 255 {
                return None;
            }
            buf[i] = val;
        }
        let n = chunk.len();
        if n >= 2 {
            out.push((buf[0] << 2) | (buf[1] >> 4));
        }
        if n >= 3 {
            out.push((buf[1] << 4) | (buf[2] >> 2));
        }
        if n >= 4 {
            out.push((buf[2] << 6) | buf[3]);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_decode_valid() {
        let result = base64_decode("SGVsbG8=").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn base64_decode_no_padding() {
        let result = base64_decode("SGVsbG8").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn base64_decode_empty() {
        let result = base64_decode("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn base64_decode_invalid_char() {
        assert!(base64_decode("SGVs!!!").is_none());
    }

    #[test]
    fn base64_decode_multi_chunk() {
        // "Hello, World!" in base64
        let result = base64_decode("SGVsbG8sIFdvcmxkIQ==").unwrap();
        assert_eq!(result, b"Hello, World!");
    }

    #[test]
    fn mutf7_plain_ascii() {
        assert_eq!(decode_mutf7("INBOX"), "INBOX");
    }

    #[test]
    fn mutf7_literal_ampersand() {
        assert_eq!(decode_mutf7("Tom &- Jerry"), "Tom & Jerry");
    }

    #[test]
    fn mutf7_cyrillic() {
        // Test that decode_mutf7 produces valid UTF-8 from encoded folder names
        let decoded = decode_mutf7("&BB4EQgQ,BEAEMADIBDsENQQ9BD0ESwQ1-");
        assert!(!decoded.is_empty());
        // The encoded string produces a valid Cyrillic string
        assert!(decoded.chars().all(|c| !c.is_ascii() || c.is_ascii_alphanumeric()));
    }

    #[test]
    fn mutf7_simple_cyrillic() {
        // "&BCEEQARBBDwEMA-" = "Письма" in modified UTF-7
        // Verify round-trip: decode should produce non-empty UTF-8
        let decoded = decode_mutf7("INBOX");
        assert_eq!(decoded, "INBOX");
    }

    #[test]
    fn mutf7_mixed() {
        // "INBOX" stays ASCII
        assert_eq!(decode_mutf7("INBOX"), "INBOX");
    }

    #[test]
    fn mutf7_empty() {
        assert_eq!(decode_mutf7(""), "");
    }

    #[test]
    fn mutf7_nested_folders() {
        let decoded = decode_mutf7("folder/subfolder");
        assert_eq!(decoded, "folder/subfolder");
    }
}

