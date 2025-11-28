//! Deprecated Centrifugo capability using the old Capabilities API.
//!
//! This module is kept for Effect enum generation via the #[derive(Effect)] macro.
//! For actual usage, prefer the Command-based API in `centrifugo_command`.

#![allow(deprecated)]

use crux_core::capability::{CapabilityContext, Operation};
use serde::{Deserialize, Serialize};

// Operations that the Shell needs to perform for Centrifugo
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CentrifugoOperation {
    Connect,
    Disconnect,
    Subscribe { channel: String },
    Unsubscribe { channel: String },
    SubscribeAll,
    UnsubscribeAll,
    History { channel: String },
}

// The output from Centrifugo operations (shell tells us what happened)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CentrifugoOutput {
    Connected,
    Disconnected,
    Subscribed {
        channel: String,
    },
    Unsubscribed {
        channel: String,
    },
    Message {
        channel: String,
        data: String,
    },
    HistoryResult {
        channel: String,
        data: Option<String>,
    },
    Error {
        message: String,
    },
}

impl Operation for CentrifugoOperation {
    type Output = CentrifugoOutput;
}

// The Centrifugo capability
pub struct Centrifugo<Ev> {
    context: CapabilityContext<CentrifugoOperation, Ev>,
}

impl<Ev> Centrifugo<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<CentrifugoOperation, Ev>) -> Self {
        Self { context }
    }

    /// Connect to Centrifugo server
    pub fn connect<F>(&self, callback: F)
    where
        F: FnOnce(CentrifugoOutput) -> Ev + Send + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let output = context
                    .request_from_shell(CentrifugoOperation::Connect)
                    .await;
                context.update_app(callback(output));
            }
        });
    }

    /// Disconnect from Centrifugo server
    pub fn disconnect<F>(&self, callback: F)
    where
        F: FnOnce(CentrifugoOutput) -> Ev + Send + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let output = context
                    .request_from_shell(CentrifugoOperation::Disconnect)
                    .await;
                context.update_app(callback(output));
            }
        });
    }

    /// Subscribe to a specific channel
    pub fn subscribe<F>(&self, channel: &str, callback: F)
    where
        F: FnOnce(CentrifugoOutput) -> Ev + Send + 'static,
    {
        let channel = channel.to_string();
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let output = context
                    .request_from_shell(CentrifugoOperation::Subscribe { channel })
                    .await;
                context.update_app(callback(output));
            }
        });
    }

    /// Unsubscribe from a specific channel
    pub fn unsubscribe<F>(&self, channel: &str, callback: F)
    where
        F: FnOnce(CentrifugoOutput) -> Ev + Send + 'static,
    {
        let channel = channel.to_string();
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let output = context
                    .request_from_shell(CentrifugoOperation::Unsubscribe { channel })
                    .await;
                context.update_app(callback(output));
            }
        });
    }

    /// Subscribe to all known channels
    pub fn subscribe_all<F>(&self, callback: F)
    where
        F: FnOnce(CentrifugoOutput) -> Ev + Send + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let output = context
                    .request_from_shell(CentrifugoOperation::SubscribeAll)
                    .await;
                context.update_app(callback(output));
            }
        });
    }

    /// Unsubscribe from all channels
    pub fn unsubscribe_all<F>(&self, callback: F)
    where
        F: FnOnce(CentrifugoOutput) -> Ev + Send + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let output = context
                    .request_from_shell(CentrifugoOperation::UnsubscribeAll)
                    .await;
                context.update_app(callback(output));
            }
        });
    }

    /// Get history (last message) from a channel
    pub fn history<F>(&self, channel: &str, callback: F)
    where
        F: FnOnce(CentrifugoOutput) -> Ev + Send + 'static,
    {
        let channel = channel.to_string();
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let output = context
                    .request_from_shell(CentrifugoOperation::History { channel })
                    .await;
                context.update_app(callback(output));
            }
        });
    }
}

impl<Ev> crux_core::Capability<Ev> for Centrifugo<Ev> {
    type Operation = CentrifugoOperation;
    type MappedSelf<MappedEv> = Centrifugo<MappedEv>;

    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + Send + Sync + 'static,
        Ev: 'static,
        NewEv: 'static + Send,
    {
        Centrifugo::new(self.context.map_event(f))
    }
}
