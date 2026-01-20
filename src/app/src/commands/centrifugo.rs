//! Centrifugo command definitions.
//!
//! These types define the interface between the Core and the Shell for Centrifugo operations.

use crux_core::{capability::Operation, command, Command};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

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

/// Command-based Centrifugo API
pub struct Centrifugo<Effect, Event> {
    _effect: PhantomData<Effect>,
    _event: PhantomData<Event>,
}

impl<Effect, Event> Centrifugo<Effect, Event>
where
    Effect: Send + From<crux_core::Request<CentrifugoOperation>> + 'static,
    Event: Send + 'static,
{
    /// Connect to Centrifugo server
    pub fn connect() -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(CentrifugoOperation::Connect)
    }

    /// Disconnect from Centrifugo server
    pub fn disconnect() -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(CentrifugoOperation::Disconnect)
    }

    /// Subscribe to a specific channel
    pub fn subscribe(channel: impl Into<String>) -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(CentrifugoOperation::Subscribe {
            channel: channel.into(),
        })
    }

    /// Unsubscribe from a specific channel
    pub fn unsubscribe(channel: impl Into<String>) -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(CentrifugoOperation::Unsubscribe {
            channel: channel.into(),
        })
    }

    /// Subscribe to all known channels
    pub fn subscribe_all() -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(CentrifugoOperation::SubscribeAll)
    }

    /// Unsubscribe from all channels
    pub fn unsubscribe_all() -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(CentrifugoOperation::UnsubscribeAll)
    }

    /// Get history (last message) from a channel
    pub fn history(channel: impl Into<String>) -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(CentrifugoOperation::History {
            channel: channel.into(),
        })
    }
}

/// Request builder for Centrifugo operations
#[must_use]
pub struct RequestBuilder<Effect, Event> {
    operation: CentrifugoOperation,
    _effect: PhantomData<Effect>,
    _event: PhantomData<fn() -> Event>,
}

impl<Effect, Event> RequestBuilder<Effect, Event>
where
    Effect: Send + From<crux_core::Request<CentrifugoOperation>> + 'static,
    Event: Send + 'static,
{
    fn new(operation: CentrifugoOperation) -> Self {
        Self {
            operation,
            _effect: PhantomData,
            _event: PhantomData,
        }
    }

    /// Build the request into a Command RequestBuilder
    pub fn build(
        self,
    ) -> command::RequestBuilder<Effect, Event, impl std::future::Future<Output = CentrifugoOutput>>
    {
        command::RequestBuilder::new(move |ctx| async move {
            Command::request_from_shell(self.operation)
                .into_future(ctx)
                .await
        })
    }
}
