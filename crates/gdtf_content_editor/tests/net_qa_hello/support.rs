use std::{error::Error, time::Duration};

use gdtf_content_editor::EditorState;
use gdtf_qa_protocol::message::QaResponse;

pub(crate) type TestError = Box<dyn Error + Send + Sync>;

pub(crate) type TestResult = Result<(), TestError>;

pub(crate) type ClientResult = Result<Vec<QaResponse>, TestError>;

pub(crate) type ReplyReport = Result<QaResponse, TestError>;

pub(crate) type TaggedReply = (QaResponse, Option<EditorState>);

pub(crate) const EDITING_EXCHANGES: usize = 3;

pub(crate) const MAX_UPDATES: u32 = 400;

pub(crate) const EDITING_UPDATES: u32 = 10_000;

pub(crate) const POLL_STEP: Duration = Duration::from_millis(10);

pub(crate) const OPEN_WAIT: Duration = Duration::from_secs(10);
