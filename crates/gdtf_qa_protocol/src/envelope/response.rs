//! [`QaResponse`] — the server-to-client response vocabulary (GTW-734).

use serde::{Deserialize, Serialize};

use super::{
    error::QaError,
    focus::FocusControlReceipt,
    hello::HelloFacts,
    menu::MenuActivationReceipt,
    receipt::InjectReceipt,
    screenshot::{ScreenshotAfterResult, ScreenshotResult},
    stepper::StepperReceipt,
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
/// (the capture result), [`ScreenshotAfter`](Self::ScreenshotAfter) (the deferred
/// frame-exact capture's folded intent + capture outcome), [`Output`](Self::Output)
/// (the drained event batch), [`StepperControlled`](Self::StepperControlled) (the DEV
/// stepper-control receipt), [`MenuItemActivated`](Self::MenuItemActivated) (the menu-item
/// activation receipt), [`FocusControlled`](Self::FocusControlled) (the focus-control
/// receipt), and [`Error`](Self::Error) (a protocol-level
/// [`QaError`]). An independent serde enum.
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
    /// The `ScreenshotAfter` result: the embedded intent's rejection, or the deferred
    /// capture's outcome.
    ScreenshotAfter(ScreenshotAfterResult),
    /// The drained event batch.
    Output(EventBatch),
    /// The DEV stepper-control command's receipt (GTW-766).
    StepperControlled(StepperReceipt),
    /// The menu-item activation receipt (GTW-787).
    MenuItemActivated(MenuActivationReceipt),
    /// The focus-control command's receipt (GTW-802).
    FocusControlled(FocusControlReceipt),
    /// A protocol-level error.
    Error(QaError),
}
