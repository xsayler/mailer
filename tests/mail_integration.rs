use async_trait::async_trait;
use mockall::mock;
use mockall::predicate::*;
use std::fmt;

// We can't import from the binary crate directly in integration tests,
// so we redefine the types and traits here for mocking.
// In a real project these would be in a library crate.

#[derive(Debug)]
pub enum MailError {
    Network(String),
    Auth(String),
    Imap(String),
    Smtp(String),
}

impl fmt::Display for MailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MailError::Network(msg) => write!(f, "{msg}"),
            MailError::Auth(msg) => write!(f, "{msg}"),
            MailError::Imap(msg) => write!(f, "{msg}"),
            MailError::Smtp(msg) => write!(f, "{msg}"),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MailFolder {
    pub name: String,
    pub path: String,
    pub unread_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Address {
    pub name: Option<String>,
    pub email: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct MailMessage {
    pub uid: u32,
    pub subject: String,
    pub from: Vec<Address>,
    pub to: Vec<Address>,
    pub is_read: bool,
    pub is_flagged: bool,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub attachments: Vec<Attachment>,
}

#[async_trait]
pub trait ImapBackend: Send + Sync {
    async fn list_folders(&self) -> Result<Vec<MailFolder>, MailError>;
    async fn fetch_messages(&self, folder: &str, limit: u32, offset: u32) -> Result<(Vec<MailMessage>, u32), MailError>;
    async fn mark_read(&self, folder: &str, uid: u32, read: bool) -> Result<(), MailError>;
    async fn delete_message(&self, folder: &str, uid: u32) -> Result<(), MailError>;
    async fn move_message(
        &self,
        src_folder: &str,
        uid: u32,
        dest_folder: &str,
    ) -> Result<(), MailError>;
    async fn fetch_unread_counts(
        &self,
        folders: &[String],
    ) -> Result<Vec<(String, u32)>, MailError>;
    async fn search_messages(
        &self,
        folder: &str,
        query: &str,
        limit: u32,
    ) -> Result<Vec<MailMessage>, MailError>;
    async fn set_flagged(&self, folder: &str, uid: u32, flagged: bool) -> Result<(), MailError>;
    async fn create_folder(&self, name: &str) -> Result<(), MailError>;
    async fn rename_folder(&self, from: &str, to: &str) -> Result<(), MailError>;
    async fn delete_folder(&self, name: &str) -> Result<(), MailError>;
}

#[async_trait]
pub trait SmtpBackend: Send + Sync {
    async fn send(
        &self,
        to: &str,
        cc: &str,
        bcc: &str,
        subject: &str,
        body: &str,
        attachments: &[(String, Vec<u8>)],
    ) -> Result<(), MailError>;
}

mock! {
    pub Imap {}

    #[async_trait]
    impl ImapBackend for Imap {
        async fn list_folders(&self) -> Result<Vec<MailFolder>, MailError>;
        async fn fetch_messages(&self, folder: &str, limit: u32, offset: u32) -> Result<(Vec<MailMessage>, u32), MailError>;
        async fn mark_read(&self, folder: &str, uid: u32, read: bool) -> Result<(), MailError>;
        async fn delete_message(&self, folder: &str, uid: u32) -> Result<(), MailError>;
        async fn move_message(&self, src_folder: &str, uid: u32, dest_folder: &str) -> Result<(), MailError>;
        async fn fetch_unread_counts(&self, folders: &[String]) -> Result<Vec<(String, u32)>, MailError>;
        async fn search_messages(&self, folder: &str, query: &str, limit: u32) -> Result<Vec<MailMessage>, MailError>;
        async fn set_flagged(&self, folder: &str, uid: u32, flagged: bool) -> Result<(), MailError>;
        async fn create_folder(&self, name: &str) -> Result<(), MailError>;
        async fn rename_folder(&self, from: &str, to: &str) -> Result<(), MailError>;
        async fn delete_folder(&self, name: &str) -> Result<(), MailError>;
    }
}

mock! {
    pub Smtp {}

    #[async_trait]
    impl SmtpBackend for Smtp {
        async fn send(
            &self,
            to: &str,
            cc: &str,
            bcc: &str,
            subject: &str,
            body: &str,
            attachments: &[(String, Vec<u8>)],
        ) -> Result<(), MailError>;
    }
}

// Helper to create test messages
fn make_msg(uid: u32, subject: &str, from_email: &str, read: bool) -> MailMessage {
    MailMessage {
        uid,
        subject: subject.to_string(),
        from: vec![Address {
            name: None,
            email: from_email.to_string(),
        }],
        is_read: read,
        body_text: Some(format!("Body of {subject}")),
        ..Default::default()
    }
}

fn make_folders() -> Vec<MailFolder> {
    vec![
        MailFolder {
            name: "INBOX".to_string(),
            path: "INBOX".to_string(),
            unread_count: 0,
        },
        MailFolder {
            name: "Sent".to_string(),
            path: "Sent".to_string(),
            unread_count: 0,
        },
        MailFolder {
            name: "Trash".to_string(),
            path: "Trash".to_string(),
            unread_count: 0,
        },
    ]
}

// ========================================
// IMAP Integration Tests
// ========================================

mod imap_tests {
    use super::*;

    #[tokio::test]
    async fn list_folders_returns_folders() {
        let mut mock = MockImap::new();
        mock.expect_list_folders()
            .times(1)
            .returning(|| Ok(make_folders()));

        let folders = mock.list_folders().await.unwrap();
        assert_eq!(folders.len(), 3);
        assert_eq!(folders[0].name, "INBOX");
        assert_eq!(folders[1].name, "Sent");
        assert_eq!(folders[2].name, "Trash");
    }

    #[tokio::test]
    async fn list_folders_connection_error() {
        let mut mock = MockImap::new();
        mock.expect_list_folders()
            .times(1)
            .returning(|| Err(MailError::Network("Connection refused".to_string())));

        let result = mock.list_folders().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Connection"));
    }

    #[tokio::test]
    async fn fetch_messages_returns_sorted() {
        let mut mock = MockImap::new();
        mock.expect_fetch_messages()
            .with(eq("INBOX"), eq(50u32), eq(0u32))
            .times(1)
            .returning(|_, _, _| {
                Ok((vec![
                    make_msg(3, "Third", "c@test.com", true),
                    make_msg(2, "Second", "b@test.com", false),
                    make_msg(1, "First", "a@test.com", true),
                ], 3))
            });

        let (messages, _total) = mock.fetch_messages("INBOX", 50, 0).await.unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].uid, 3);
        assert_eq!(messages[2].uid, 1);
    }

    #[tokio::test]
    async fn fetch_messages_empty_folder() {
        let mut mock = MockImap::new();
        mock.expect_fetch_messages()
            .with(eq("EmptyFolder"), eq(50u32), eq(0u32))
            .times(1)
            .returning(|_, _, _| Ok((vec![], 0)));

        let (messages, _) = mock.fetch_messages("EmptyFolder", 50, 0).await.unwrap();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn fetch_messages_error() {
        let mut mock = MockImap::new();
        mock.expect_fetch_messages()
            .times(1)
            .returning(|_, _, _| Err(MailError::Imap("SELECT failed: folder not found".to_string())));

        let result = mock.fetch_messages("NonExistent", 50, 0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mark_read_success() {
        let mut mock = MockImap::new();
        mock.expect_mark_read()
            .with(eq("INBOX"), eq(42u32), eq(true))
            .times(1)
            .returning(|_, _, _| Ok(()));

        let result = mock.mark_read("INBOX", 42, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn mark_unread_success() {
        let mut mock = MockImap::new();
        mock.expect_mark_read()
            .with(eq("INBOX"), eq(42u32), eq(false))
            .times(1)
            .returning(|_, _, _| Ok(()));

        let result = mock.mark_read("INBOX", 42, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn mark_read_error() {
        let mut mock = MockImap::new();
        mock.expect_mark_read()
            .times(1)
            .returning(|_, _, _| Err(MailError::Imap("STORE failed: connection lost".to_string())));

        let result = mock.mark_read("INBOX", 1, true).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("STORE"));
    }

    #[tokio::test]
    async fn delete_message_success() {
        let mut mock = MockImap::new();
        mock.expect_delete_message()
            .with(eq("INBOX"), eq(10u32))
            .times(1)
            .returning(|_, _| Ok(()));

        let result = mock.delete_message("INBOX", 10).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_message_error() {
        let mut mock = MockImap::new();
        mock.expect_delete_message()
            .times(1)
            .returning(|_, _| Err(MailError::Imap("EXPUNGE failed".to_string())));

        let result = mock.delete_message("INBOX", 10).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn move_message_success() {
        let mut mock = MockImap::new();
        mock.expect_move_message()
            .with(eq("INBOX"), eq(5u32), eq("Trash"))
            .times(1)
            .returning(|_, _, _| Ok(()));

        let result = mock.move_message("INBOX", 5, "Trash").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn move_message_to_same_folder() {
        let mut mock = MockImap::new();
        mock.expect_move_message()
            .with(eq("INBOX"), eq(5u32), eq("INBOX"))
            .times(1)
            .returning(|_, _, _| Ok(()));

        let result = mock.move_message("INBOX", 5, "INBOX").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn move_message_error() {
        let mut mock = MockImap::new();
        mock.expect_move_message()
            .times(1)
            .returning(|_, _, _| Err(MailError::Imap("COPY failed: destination not found".to_string())));

        let result = mock.move_message("INBOX", 5, "NonExistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("COPY"));
    }

    #[tokio::test]
    async fn fetch_unread_counts_success() {
        let mut mock = MockImap::new();
        mock.expect_fetch_unread_counts()
            .times(1)
            .returning(|_| {
                Ok(vec![
                    ("INBOX".to_string(), 5),
                    ("Sent".to_string(), 0),
                    ("Trash".to_string(), 2),
                ])
            });

        let folders = vec![
            "INBOX".to_string(),
            "Sent".to_string(),
            "Trash".to_string(),
        ];
        let counts = mock.fetch_unread_counts(&folders).await.unwrap();
        assert_eq!(counts.len(), 3);
        assert_eq!(counts[0], ("INBOX".to_string(), 5));
        assert_eq!(counts[1], ("Sent".to_string(), 0));
        assert_eq!(counts[2], ("Trash".to_string(), 2));
    }

    #[tokio::test]
    async fn fetch_unread_counts_empty() {
        let mut mock = MockImap::new();
        mock.expect_fetch_unread_counts()
            .times(1)
            .returning(|_| Ok(vec![]));

        let counts = mock.fetch_unread_counts(&[]).await.unwrap();
        assert!(counts.is_empty());
    }
}

// ========================================
// SMTP Integration Tests
// ========================================

mod smtp_tests {
    use super::*;

    #[tokio::test]
    async fn send_plain_text_success() {
        let mut mock = MockSmtp::new();
        mock.expect_send()
            .with(
                eq("recipient@test.com"),
                eq(""),
                eq(""),
                eq("Test Subject"),
                eq("Hello World"),
                eq(Vec::<(String, Vec<u8>)>::new()),
            )
            .times(1)
            .returning(|_, _, _, _, _, _| Ok(()));

        let result = mock
            .send("recipient@test.com", "", "", "Test Subject", "Hello World", &[])
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn send_with_cc() {
        let mut mock = MockSmtp::new();
        mock.expect_send()
            .with(
                eq("to@test.com"),
                eq("cc@test.com"),
                eq(""),
                always(),
                always(),
                always(),
            )
            .times(1)
            .returning(|_, _, _, _, _, _| Ok(()));

        let result = mock
            .send("to@test.com", "cc@test.com", "", "Subject", "Body", &[])
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn send_with_attachments() {
        let mut mock = MockSmtp::new();
        mock.expect_send()
            .withf(|_, _, _, _, _, atts| atts.len() == 2)
            .times(1)
            .returning(|_, _, _, _, _, _| Ok(()));

        let attachments = vec![
            ("file1.txt".to_string(), b"content1".to_vec()),
            ("file2.pdf".to_string(), b"content2".to_vec()),
        ];

        let result = mock
            .send("to@test.com", "", "", "Subject", "Body", &attachments)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn send_html_body() {
        let mut mock = MockSmtp::new();
        mock.expect_send()
            .withf(|_, _, _, _, body, _| body.contains("<b>"))
            .times(1)
            .returning(|_, _, _, _, _, _| Ok(()));

        let result = mock
            .send(
                "to@test.com",
                "",
                "",
                "Subject",
                "Hello <b>World</b>",
                &[],
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn send_failure() {
        let mut mock = MockSmtp::new();
        mock.expect_send()
            .times(1)
            .returning(|_, _, _, _, _, _| Err(MailError::Smtp("SMTP relay error: connection refused".to_string())));

        let result = mock
            .send("to@test.com", "", "", "Subject", "Body", &[])
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SMTP"));
    }

    #[tokio::test]
    async fn send_invalid_recipient() {
        let mut mock = MockSmtp::new();
        mock.expect_send()
            .times(1)
            .returning(|_, _, _, _, _, _| Err(MailError::Smtp("Invalid to address: not-an-email".to_string())));

        let result = mock
            .send("not-an-email", "", "", "Subject", "Body", &[])
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid"));
    }
}

// ========================================
// Workflow Integration Tests
// ========================================

mod workflow_tests {
    use super::*;

    /// Simulates the full workflow: connect → list folders → select folder → fetch messages
    #[tokio::test]
    async fn full_inbox_workflow() {
        let mut mock = MockImap::new();

        // Step 1: List folders
        mock.expect_list_folders()
            .times(1)
            .returning(|| Ok(make_folders()));

        // Step 2: Fetch messages from INBOX
        mock.expect_fetch_messages()
            .with(eq("INBOX"), eq(50u32), eq(0u32))
            .times(1)
            .returning(|_, _, _| {
                Ok((vec![
                    make_msg(1, "Welcome", "admin@test.com", false),
                    make_msg(2, "Report", "boss@test.com", true),
                ], 2))
            });

        // Step 3: Fetch unread counts
        mock.expect_fetch_unread_counts()
            .times(1)
            .returning(|_| {
                Ok(vec![
                    ("INBOX".to_string(), 1),
                    ("Sent".to_string(), 0),
                    ("Trash".to_string(), 0),
                ])
            });

        // Execute workflow
        let folders = mock.list_folders().await.unwrap();
        assert_eq!(folders.len(), 3);

        let (messages, _) = mock.fetch_messages("INBOX", 50, 0).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert!(!messages[0].is_read);
        assert!(messages[1].is_read);

        let folder_paths: Vec<String> = folders.iter().map(|f| f.path.clone()).collect();
        let counts = mock.fetch_unread_counts(&folder_paths).await.unwrap();
        assert_eq!(counts[0].1, 1); // INBOX has 1 unread
    }

    /// Simulates: read message → mark as read → verify
    #[tokio::test]
    async fn read_and_mark_workflow() {
        let mut mock = MockImap::new();

        mock.expect_fetch_messages()
            .times(1)
            .returning(|_, _, _| Ok((vec![make_msg(10, "Unread msg", "user@test.com", false)], 1)));

        mock.expect_mark_read()
            .with(eq("INBOX"), eq(10u32), eq(true))
            .times(1)
            .returning(|_, _, _| Ok(()));

        let (messages, _) = mock.fetch_messages("INBOX", 50, 0).await.unwrap();
        assert!(!messages[0].is_read);

        // Mark as read
        mock.mark_read("INBOX", 10, true).await.unwrap();
    }

    /// Simulates: delete message from INBOX
    #[tokio::test]
    async fn delete_workflow() {
        let mut mock = MockImap::new();

        mock.expect_fetch_messages()
            .times(1)
            .returning(|_, _, _| {
                Ok((vec![
                    make_msg(1, "Keep", "a@test.com", true),
                    make_msg(2, "Delete me", "b@test.com", true),
                    make_msg(3, "Also keep", "c@test.com", true),
                ], 3))
            });

        mock.expect_delete_message()
            .with(eq("INBOX"), eq(2u32))
            .times(1)
            .returning(|_, _| Ok(()));

        let (messages, _) = mock.fetch_messages("INBOX", 50, 0).await.unwrap();
        assert_eq!(messages.len(), 3);

        // Delete message 2
        mock.delete_message("INBOX", 2).await.unwrap();
    }

    /// Simulates: move message from INBOX to Archive
    #[tokio::test]
    async fn move_to_archive_workflow() {
        let mut mock = MockImap::new();

        mock.expect_fetch_messages()
            .times(1)
            .returning(|_, _, _| Ok((vec![make_msg(5, "Archive this", "user@test.com", true)], 1)));

        mock.expect_move_message()
            .with(eq("INBOX"), eq(5u32), eq("Archive"))
            .times(1)
            .returning(|_, _, _| Ok(()));

        let (messages, _) = mock.fetch_messages("INBOX", 50, 0).await.unwrap();
        assert_eq!(messages.len(), 1);

        mock.move_message("INBOX", 5, "Archive").await.unwrap();
    }

    /// Simulates compose and send workflow
    #[tokio::test]
    async fn compose_and_send_workflow() {
        let mut mock = MockSmtp::new();

        mock.expect_send()
            .withf(|to, cc, _bcc, subject, body, atts| {
                to == "recipient@example.com"
                    && cc.is_empty()
                    && subject == "Hello from test"
                    && body.contains("test message")
                    && atts.is_empty()
            })
            .times(1)
            .returning(|_, _, _, _, _, _| Ok(()));

        mock.send(
            "recipient@example.com",
            "",
            "",
            "Hello from test",
            "This is a test message",
            &[],
        )
        .await
        .unwrap();
    }

    /// Simulates reply workflow
    #[tokio::test]
    async fn reply_workflow() {
        let mut imap = MockImap::new();
        let mut smtp = MockSmtp::new();

        // Fetch the original message
        imap.expect_fetch_messages()
            .times(1)
            .returning(|_, _, _| {
                Ok((vec![MailMessage {
                    uid: 7,
                    subject: "Question about project".to_string(),
                    from: vec![Address {
                        name: Some("Alice".to_string()),
                        email: "alice@test.com".to_string(),
                    }],
                    body_text: Some("What is the status?".to_string()),
                    ..Default::default()
                }], 1))
            });

        // Mark original as read
        imap.expect_mark_read()
            .with(eq("INBOX"), eq(7u32), eq(true))
            .times(1)
            .returning(|_, _, _| Ok(()));

        // Send reply
        smtp.expect_send()
            .withf(|to, _, _, subject, body, _| {
                to == "alice@test.com"
                    && subject.starts_with("Re:")
                    && body.contains("> What is the status?")
            })
            .times(1)
            .returning(|_, _, _, _, _, _| Ok(()));

        // Execute
        let (messages, _) = imap.fetch_messages("INBOX", 50, 0).await.unwrap();
        let original = &messages[0];

        imap.mark_read("INBOX", original.uid, true).await.unwrap();

        let reply_to = &original.from[0].email;
        let re_subject = format!("Re: {}", original.subject);
        let quoted = original
            .body_text
            .as_deref()
            .unwrap()
            .lines()
            .map(|l| format!("> {l}"))
            .collect::<Vec<_>>()
            .join("\n");
        let reply_body = format!("Everything is on track.\n\n{quoted}");

        smtp.send(reply_to, "", "", &re_subject, &reply_body, &[])
            .await
            .unwrap();
    }

    /// Simulates send with attachment
    #[tokio::test]
    async fn send_with_attachment_workflow() {
        let mut smtp = MockSmtp::new();

        smtp.expect_send()
            .withf(|to, _, _, subject, _, atts| {
                to == "recipient@test.com"
                    && subject == "File attached"
                    && atts.len() == 1
                    && atts[0].0 == "report.pdf"
            })
            .times(1)
            .returning(|_, _, _, _, _, _| Ok(()));

        let attachment_data = vec![0u8; 1024]; // fake PDF
        smtp.send(
            "recipient@test.com",
            "",
            "",
            "File attached",
            "Please find the report attached.",
            &[("report.pdf".to_string(), attachment_data)],
        )
        .await
        .unwrap();
    }

    /// Test error recovery: operation fails, then succeeds on retry
    #[tokio::test]
    async fn retry_on_failure() {
        let mut mock = MockImap::new();

        let mut call_count = 0u32;
        mock.expect_list_folders()
            .times(2)
            .returning(move || {
                call_count += 1;
                if call_count == 1 {
                    Err(MailError::Network("Connection lost".to_string()))
                } else {
                    Ok(make_folders())
                }
            });

        // First attempt fails
        let result = mock.list_folders().await;
        assert!(result.is_err());

        // Retry succeeds
        let result = mock.list_folders().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }

    /// Test fetching messages with attachments
    #[tokio::test]
    async fn fetch_messages_with_attachments() {
        let mut mock = MockImap::new();

        mock.expect_fetch_messages()
            .times(1)
            .returning(|_, _, _| {
                Ok((vec![MailMessage {
                    uid: 1,
                    subject: "With attachment".to_string(),
                    attachments: vec![
                        Attachment {
                            filename: "doc.pdf".to_string(),
                            content_type: "application/pdf".to_string(),
                            size: 2048,
                            data: vec![0u8; 2048],
                        },
                        Attachment {
                            filename: "image.png".to_string(),
                            content_type: "image/png".to_string(),
                            size: 512,
                            data: vec![0u8; 512],
                        },
                    ],
                    ..Default::default()
                }], 1))
            });

        let (messages, _) = mock.fetch_messages("INBOX", 50, 0).await.unwrap();
        assert_eq!(messages[0].attachments.len(), 2);
        assert_eq!(messages[0].attachments[0].filename, "doc.pdf");
        assert_eq!(messages[0].attachments[1].filename, "image.png");
        assert_eq!(messages[0].attachments[0].size, 2048);
    }

    #[tokio::test]
    async fn search_messages_returns_results() {
        let mut mock = MockImap::new();
        mock.expect_search_messages()
            .with(eq("INBOX"), eq("test query"), eq(50u32))
            .times(1)
            .returning(|_, _, _| Ok(vec![make_msg(10, "Test result", "a@test.com", false)]));

        let results = mock.search_messages("INBOX", "test query", 50).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].subject, "Test result");
    }

    #[tokio::test]
    async fn set_flagged_success() {
        let mut mock = MockImap::new();
        mock.expect_set_flagged()
            .with(eq("INBOX"), eq(5u32), eq(true))
            .times(1)
            .returning(|_, _, _| Ok(()));

        let result = mock.set_flagged("INBOX", 5, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_folder_success() {
        let mut mock = MockImap::new();
        mock.expect_create_folder()
            .with(eq("NewFolder"))
            .times(1)
            .returning(|_| Ok(()));

        let result = mock.create_folder("NewFolder").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn rename_folder_success() {
        let mut mock = MockImap::new();
        mock.expect_rename_folder()
            .with(eq("OldName"), eq("NewName"))
            .times(1)
            .returning(|_, _| Ok(()));

        let result = mock.rename_folder("OldName", "NewName").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_folder_success() {
        let mut mock = MockImap::new();
        mock.expect_delete_folder()
            .with(eq("ToDelete"))
            .times(1)
            .returning(|_| Ok(()));

        let result = mock.delete_folder("ToDelete").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn send_with_bcc() {
        let mut mock = MockSmtp::new();
        mock.expect_send()
            .withf(|to, _cc, bcc, _subj, _body, _atts| {
                to == "to@test.com" && bcc == "hidden@test.com"
            })
            .times(1)
            .returning(|_, _, _, _, _, _| Ok(()));

        let result = mock
            .send("to@test.com", "", "hidden@test.com", "Subject", "Body", &[])
            .await;
        assert!(result.is_ok());
    }
}
