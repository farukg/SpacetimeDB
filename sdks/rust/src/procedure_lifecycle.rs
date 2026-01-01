use crate::{error::InternalError, Error};
use spacetimedb_data_structures::map::{hash_map::Entry, HashMap};

#[derive(Clone, Debug)]
pub(crate) enum ConnectionEnd {
    Requested,
    TransportClosed,
    TransportFailed(Error),
}

impl ConnectionEnd {
    pub(crate) fn callback_error(&self) -> InternalError {
        InternalError::new("Procedure call ended before receiving a result").with_cause(self.return_error())
    }

    pub(crate) fn return_error(&self) -> Error {
        match self {
            Self::Requested | Self::TransportClosed => Error::Disconnected,
            Self::TransportFailed(error) => error.clone(),
        }
    }

    pub(crate) fn public_error(&self) -> Option<Error> {
        match self {
            Self::Requested | Self::TransportClosed => None,
            Self::TransportFailed(error) => Some(error.clone()),
        }
    }
}

pub(crate) enum ConnectionActivity {
    Active,
    Ended,
}

pub(crate) enum ConnectTransition {
    Established,
    AlreadyConnected,
    AlreadyEnded,
}

pub(crate) enum ProcedureRegistration<C> {
    Stored,
    RejectedAfterEnd { callback: C, end: ConnectionEnd },
    DuplicateRequestId(C),
}

pub(crate) enum ProcedureResolution<C> {
    Deliver(C),
    LateAfterEnd,
    UnknownRequest,
}

pub(crate) enum ConnectionNotification {
    Suppressed,
    ConnectError(Error),
    Disconnected(Option<Error>),
}

pub(crate) struct ConnectionTermination<C> {
    pub(crate) callbacks: Vec<C>,
    pub(crate) end: ConnectionEnd,
    pub(crate) notification: ConnectionNotification,
}

pub(crate) enum TerminationTransition<C> {
    Ended(ConnectionTermination<C>),
    AlreadyEnded(ConnectionEnd),
}

pub(crate) enum RequestedDisconnect<C> {
    Accepted(ConnectionTermination<C>),
    AlreadyEnded,
}

pub(crate) struct ProcedureRegistry<C> {
    callbacks: HashMap<u32, C>,
}

impl<C> Default for ProcedureRegistry<C> {
    fn default() -> Self {
        Self {
            callbacks: HashMap::default(),
        }
    }
}

impl<C> ProcedureRegistry<C> {
    fn register(&mut self, request_id: u32, callback: C) -> ProcedureRegistration<C> {
        match self.callbacks.entry(request_id) {
            Entry::Vacant(entry) => {
                entry.insert(callback);
                ProcedureRegistration::Stored
            }
            Entry::Occupied(_) => ProcedureRegistration::DuplicateRequestId(callback),
        }
    }

    fn resolve(&mut self, request_id: u32) -> ProcedureResolution<C> {
        match self.callbacks.remove(&request_id) {
            Some(callback) => ProcedureResolution::Deliver(callback),
            None => ProcedureResolution::UnknownRequest,
        }
    }

    fn take_all(&mut self) -> Vec<C> {
        self.callbacks.drain().map(|(_, callback)| callback).collect()
    }
}

pub(crate) enum ConnectionLifecycle<C> {
    Connecting(ProcedureRegistry<C>),
    Connected(ProcedureRegistry<C>),
    Ended(ConnectionEnd),
}

impl<C> Default for ConnectionLifecycle<C> {
    fn default() -> Self {
        Self::Connecting(ProcedureRegistry::default())
    }
}

impl<C> ConnectionLifecycle<C> {
    pub(crate) fn activity(&self) -> ConnectionActivity {
        match self {
            Self::Connecting(_) | Self::Connected(_) => ConnectionActivity::Active,
            Self::Ended(_) => ConnectionActivity::Ended,
        }
    }

    pub(crate) fn establish(&mut self) -> ConnectTransition {
        match self {
            Self::Connecting(callbacks) => {
                let callbacks = std::mem::take(callbacks);
                *self = Self::Connected(callbacks);
                ConnectTransition::Established
            }
            Self::Connected(_) => ConnectTransition::AlreadyConnected,
            Self::Ended(_) => ConnectTransition::AlreadyEnded,
        }
    }

    pub(crate) fn register(&mut self, request_id: u32, callback: C) -> ProcedureRegistration<C> {
        match self {
            Self::Connecting(callbacks) | Self::Connected(callbacks) => callbacks.register(request_id, callback),
            Self::Ended(end) => ProcedureRegistration::RejectedAfterEnd {
                callback,
                end: end.clone(),
            },
        }
    }

    pub(crate) fn resolve(&mut self, request_id: u32) -> ProcedureResolution<C> {
        match self {
            Self::Connecting(callbacks) | Self::Connected(callbacks) => callbacks.resolve(request_id),
            Self::Ended(_) => ProcedureResolution::LateAfterEnd,
        }
    }

    pub(crate) fn request_disconnect(&mut self) -> RequestedDisconnect<C> {
        match self.terminate(ConnectionEnd::Requested) {
            TerminationTransition::Ended(termination) => RequestedDisconnect::Accepted(termination),
            TerminationTransition::AlreadyEnded(_) => RequestedDisconnect::AlreadyEnded,
        }
    }

    pub(crate) fn terminate(&mut self, end: ConnectionEnd) -> TerminationTransition<C> {
        let lifecycle = std::mem::replace(self, Self::Ended(end.clone()));
        match lifecycle {
            Self::Connecting(mut callbacks) => {
                let notification = match &end {
                    ConnectionEnd::Requested => ConnectionNotification::Suppressed,
                    ConnectionEnd::TransportClosed => ConnectionNotification::ConnectError(Error::FailedToConnect {
                        source: InternalError::new("Connection closed before receiving the initial connection message"),
                    }),
                    ConnectionEnd::TransportFailed(error) => ConnectionNotification::ConnectError(error.clone()),
                };
                TerminationTransition::Ended(ConnectionTermination {
                    callbacks: callbacks.take_all(),
                    end,
                    notification,
                })
            }
            Self::Connected(mut callbacks) => {
                let notification = ConnectionNotification::Disconnected(end.public_error());
                TerminationTransition::Ended(ConnectionTermination {
                    callbacks: callbacks.take_all(),
                    end,
                    notification,
                })
            }
            Self::Ended(first_end) => {
                *self = Self::Ended(first_end.clone());
                TerminationTransition::AlreadyEnded(first_end)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        error::Error as _,
        sync::{Arc, Barrier, Mutex},
        thread,
    };

    #[test]
    fn requested_disconnect_is_linearizable() {
        let mut lifecycle = ConnectionLifecycle::default();
        assert!(matches!(lifecycle.establish(), ConnectTransition::Established));
        let lifecycle = Arc::new(Mutex::new(lifecycle));
        let barrier = Arc::new(Barrier::new(3));
        let first = {
            let lifecycle = Arc::clone(&lifecycle);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                lifecycle.lock().unwrap().request_disconnect()
            })
        };
        let second = {
            let lifecycle = Arc::clone(&lifecycle);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                lifecycle.lock().unwrap().request_disconnect()
            })
        };
        barrier.wait();
        let outcomes = [first.join().unwrap(), second.join().unwrap()];
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(outcome, RequestedDisconnect::Accepted(_)))
                .count(),
            1
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(outcome, RequestedDisconnect::AlreadyEnded))
                .count(),
            1
        );
    }

    #[test]
    fn protocol_result_and_connection_end_transfer_each_callback_once() {
        let mut lifecycle = ConnectionLifecycle::default();
        assert!(matches!(lifecycle.establish(), ConnectTransition::Established));
        assert!(matches!(lifecycle.register(11, 11), ProcedureRegistration::Stored));
        assert!(matches!(lifecycle.register(12, 12), ProcedureRegistration::Stored));

        let lifecycle = Arc::new(Mutex::new(lifecycle));
        let barrier = Arc::new(Barrier::new(3));
        let result_thread = {
            let lifecycle = Arc::clone(&lifecycle);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                lifecycle.lock().unwrap().resolve(11)
            })
        };
        let termination_thread = {
            let lifecycle = Arc::clone(&lifecycle);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                lifecycle.lock().unwrap().terminate(ConnectionEnd::TransportClosed)
            })
        };
        barrier.wait();

        let resolution = result_thread.join().unwrap();
        let termination = termination_thread.join().unwrap();
        let mut transferred = Vec::new();
        match resolution {
            ProcedureResolution::Deliver(callback) => transferred.push(callback),
            ProcedureResolution::LateAfterEnd => {}
            ProcedureResolution::UnknownRequest => panic!("registered request became unknown"),
        }
        match termination {
            TerminationTransition::Ended(termination) => transferred.extend(termination.callbacks),
            TerminationTransition::AlreadyEnded(_) => {}
        }
        transferred.sort_unstable();
        assert_eq!(transferred, [11, 12]);

        let mut lifecycle = lifecycle.lock().unwrap();
        assert!(matches!(
            lifecycle.terminate(ConnectionEnd::Requested),
            TerminationTransition::AlreadyEnded(ConnectionEnd::TransportClosed)
        ));
        assert!(matches!(lifecycle.resolve(11), ProcedureResolution::LateAfterEnd));
        let ProcedureRegistration::RejectedAfterEnd { callback, end } = lifecycle.register(13, 13) else {
            panic!("post-terminal registration was retained")
        };
        assert_eq!(callback, 13);
        let error = end.callback_error();
        assert!(matches!(
            error.cause().and_then(|cause| cause.downcast_ref::<Error>()),
            Some(Error::Disconnected)
        ));
    }
}
