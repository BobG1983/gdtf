//! [`QaResponse`] — the server-to-client response vocabulary (GTW-734).

use serde::{Deserialize, Serialize};

use super::{
    error::QaError, hello::HelloFacts, receipt::InjectReceipt, screenshot::ScreenshotResult,
};
use crate::{
    events::EventBatch,
    view::{AppFlowView, BattleView},
};

/// A single reply the game's `net_qa` server returns for a
/// [`QaRequest`](crate::envelope::QaRequest).
///
/// The reply surface, one variant per successful request kind plus the catch-all
/// [`Error`](Self::Error): [`HelloOk`](Self::HelloOk) (the handshake facts),
/// [`AppFlow`](Self::AppFlow) / [`Battle`](Self::Battle) (the two snapshots),
/// [`Injected`](Self::Injected) (the inject receipt), [`Screenshot`](Self::Screenshot)
/// (the capture result), [`Output`](Self::Output) (the drained event batch), and
/// [`Error`](Self::Error) (a protocol-level [`QaError`]). An independent serde enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QaResponse {
    /// The handshake succeeded — the negotiated facts.
    HelloOk(HelloFacts),
    /// The app-flow snapshot.
    AppFlow(AppFlowView),
    /// The whole battle snapshot.
    Battle(BattleView),
    /// The injected intent's receipt.
    Injected(InjectReceipt),
    /// The screenshot result.
    Screenshot(ScreenshotResult),
    /// The drained event batch.
    Output(EventBatch),
    /// A protocol-level error.
    Error(QaError),
}
