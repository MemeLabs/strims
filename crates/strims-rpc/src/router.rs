// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Method handlers by name.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::rc::Rc;

use futures_util::StreamExt;
use futures_util::future::LocalBoxFuture;
use prost::Message;
use strims_proto::service::{Context, Error, Registry, ServerStream, StreamingMethod, UnaryMethod};

use crate::shared::{Responder, yield_now};

/// Stream items a handler sends before it yields, so its connection reads
/// input (cancels among it) between bursts of an always-ready stream.
const STREAM_BURST: usize = 32;

/// A handler, erased to an encoded request in and replies out.
pub(crate) type Handler = dyn Fn(Context, Vec<u8>, Responder) -> LocalBoxFuture<'static, ()>;

/// The handlers a connection serves, by method name. Generated
/// `register_*_service` functions fill it.
///
/// Clones share the handlers, and methods can be added while connections
/// serve them, e.g. once a caller has signed in. A call of a method that
/// isn't registered is answered with [`Error::method_not_found`].
#[derive(Clone, Default)]
pub struct Router {
    methods: Rc<RefCell<BTreeMap<&'static str, Rc<Handler>>>>,
}

impl fmt::Debug for Router {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.methods.borrow().keys()).finish()
    }
}

impl Router {
    /// A router with no methods.
    pub fn new() -> Router {
        Router::default()
    }

    /// Whether `method` is registered.
    pub fn contains(&self, method: &str) -> bool {
        self.methods.borrow().contains_key(method)
    }

    /// Stops serving `method`. Calls in service run to completion.
    pub fn remove(&self, method: &str) {
        self.methods.borrow_mut().remove(method);
    }

    pub(crate) fn get(&self, method: &str) -> Option<Rc<Handler>> {
        self.methods.borrow().get(method).cloned()
    }

    fn insert(&self, method: &'static str, handler: Rc<Handler>) {
        self.methods.borrow_mut().insert(method, handler);
    }
}

fn decode_error(e: &prost::DecodeError) -> Error {
    Error::new(0, format!("decoding the request: {e}"))
}

impl Registry for Router {
    fn register_unary<M, F, Fut>(&mut self, handler: F)
    where
        M: UnaryMethod,
        F: Fn(Context, M::Request) -> Fut + 'static,
        Fut: Future<Output = Result<M::Response, Error>> + 'static,
    {
        let handler = Rc::new(handler);
        self.insert(
            M::NAME,
            Rc::new(move |ctx, arg: Vec<u8>, responder: Responder| {
                let handler = Rc::clone(&handler);
                Box::pin(async move {
                    let req = match M::Request::decode(arg.as_slice()) {
                        Ok(req) => req,
                        Err(e) => return responder.reply_error(&decode_error(&e)),
                    };
                    match handler(ctx, req).await {
                        Ok(res) => responder.reply_value(&res),
                        Err(e) => responder.reply_error(&e),
                    }
                })
            }),
        );
    }

    fn register_streaming<M, F, Fut>(&mut self, handler: F)
    where
        M: StreamingMethod,
        F: Fn(Context, M::Request) -> Fut + 'static,
        Fut: Future<Output = Result<ServerStream<M::Response>, Error>> + 'static,
    {
        let handler = Rc::new(handler);
        self.insert(
            M::NAME,
            Rc::new(move |ctx, arg: Vec<u8>, responder: Responder| {
                let handler = Rc::clone(&handler);
                Box::pin(async move {
                    let req = match M::Request::decode(arg.as_slice()) {
                        Ok(req) => req,
                        Err(e) => return responder.reply_error(&decode_error(&e)),
                    };
                    let mut stream = match handler(ctx, req).await {
                        Ok(stream) => stream,
                        Err(e) => return responder.reply_error(&e),
                    };
                    let mut burst = 0;
                    loop {
                        // pull no more than the transport takes
                        responder.ready().await;
                        burst += 1;
                        if burst == STREAM_BURST {
                            burst = 0;
                            yield_now().await;
                        }
                        match stream.next().await {
                            Some(Ok(item)) => {
                                if !responder.reply_item(&item) {
                                    return;
                                }
                            }
                            Some(Err(e)) => return responder.reply_error(&e),
                            None => return responder.reply_close(),
                        }
                    }
                })
            }),
        );
    }
}
