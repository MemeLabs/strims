// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Futures over IndexedDB requests and transactions.
//!
//! Each future resolves from the callbacks of the request or transaction
//! it watches, which wake its task. wasm-bindgen-futures polls the task in
//! a microtask, while the transaction is still active, so the task can
//! issue its next request (see the [crate docs](crate)).

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use strims_kv::Error;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Event, IdbDatabase, IdbFactory, IdbOpenDbRequest, IdbRequest, IdbTransaction};

use crate::js;

/// The version of every table's database.
pub(crate) const VERSION: u32 = 1;
/// The object store that holds a table's entries.
pub(crate) const STORE: &str = "data";

/// Maps a thrown or reported IndexedDB error. A transaction that has
/// committed or aborted refuses requests with `TransactionInactiveError`
/// or `InvalidStateError`.
pub(crate) fn error(e: &JsValue) -> Error {
    match js::error_name(e).as_deref() {
        Some("TransactionInactiveError" | "InvalidStateError") => Error::Inactive,
        _ => Error::Backend(js::describe(e)),
    }
}

#[wasm_bindgen]
extern "C" {
    /// An `IDBTransaction`, for `commit()`, which web-sys marks deprecated
    /// although IndexedDB 3 defines it.
    type Committable;

    #[wasm_bindgen(method, catch)]
    fn commit(this: &Committable) -> Result<(), JsValue>;
}

/// Commits `tx` now, without waiting for its task to return to the event
/// loop.
pub(crate) fn commit(tx: &IdbTransaction) -> Result<(), Error> {
    tx.unchecked_ref::<Committable>()
        .commit()
        .map_err(|e| error(&e))
}

/// The error a request reports.
fn request_error(req: &IdbRequest) -> Error {
    match req.error() {
        Ok(Some(e)) => error(&e),
        Ok(None) => Error::Backend("request failed".to_owned()),
        Err(e) => error(&e),
    }
}

/// An outcome a callback fills, and the task it wakes.
struct Slot<T> {
    out: RefCell<Option<Result<T, Error>>>,
    waker: RefCell<Option<Waker>>,
}

impl<T> Slot<T> {
    fn new() -> Slot<T> {
        Slot {
            out: RefCell::new(None),
            waker: RefCell::new(None),
        }
    }

    fn finish(&self, out: Result<T, Error>) {
        let old = self.out.replace(Some(out));
        drop(old);
        let waker = self.waker.take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn poll(&self, cx: &mut Context<'_>) -> Poll<Result<T, Error>> {
        let out = self.out.take();
        match out {
            Some(out) => Poll::Ready(out),
            None => {
                let old = self.waker.replace(Some(cx.waker().clone()));
                drop(old);
                Poll::Pending
            }
        }
    }
}

/// A request's outcome.
///
/// `on_success` runs at every success event and returns the outcome, or
/// `None` while the request continues (a cursor that moved on). An error
/// event fails the request, and IndexedDB then aborts its transaction.
/// Dropping the future detaches it; the request still runs.
pub(crate) struct Request<T> {
    req: IdbRequest,
    slot: Rc<Slot<T>>,
    _on_success: Closure<dyn FnMut()>,
    _on_error: Closure<dyn FnMut()>,
    on_blocked: Option<Closure<dyn FnMut()>>,
}

impl<T: 'static> Request<T> {
    pub(crate) fn new(
        req: IdbRequest,
        mut on_success: impl FnMut(&IdbRequest) -> Option<Result<T, Error>> + 'static,
    ) -> Request<T> {
        let slot = Rc::new(Slot::new());
        let on_success = {
            let (req, slot) = (req.clone(), slot.clone());
            Closure::<dyn FnMut()>::new(move || {
                if let Some(out) = on_success(&req) {
                    slot.finish(out);
                }
            })
        };
        let on_error = {
            let (req, slot) = (req.clone(), slot.clone());
            Closure::<dyn FnMut()>::new(move || slot.finish(Err(request_error(&req))))
        };
        req.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));
        req.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        Request {
            req,
            slot,
            _on_success: on_success,
            _on_error: on_error,
            on_blocked: None,
        }
    }

    /// Runs `f` at each `blocked` event of a database request.
    pub(crate) fn on_blocked(mut self, f: impl FnMut() + 'static) -> Request<T> {
        let f = Closure::<dyn FnMut()>::new(f);
        self.req
            .unchecked_ref::<IdbOpenDbRequest>()
            .set_onblocked(Some(f.as_ref().unchecked_ref()));
        self.on_blocked = Some(f);
        self
    }
}

/// A request's success value.
pub(crate) fn request(req: IdbRequest) -> Request<JsValue> {
    Request::new(req, |r| Some(r.result().map_err(|e| error(&e))))
}

impl<T> Future for Request<T> {
    type Output = Result<T, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.slot.poll(cx)
    }
}

impl<T> Drop for Request<T> {
    fn drop(&mut self) {
        self.req.set_onsuccess(None);
        self.req.set_onerror(None);
        if self.on_blocked.is_some() {
            self.req
                .unchecked_ref::<IdbOpenDbRequest>()
                .set_onblocked(None);
        }
    }
}

/// A transaction's `complete`, or the error that aborted it.
pub(crate) struct Completion {
    tx: IdbTransaction,
    slot: Rc<Slot<()>>,
    _on_complete: Closure<dyn FnMut()>,
    _on_abort: Closure<dyn FnMut()>,
}

impl Completion {
    pub(crate) fn new(tx: &IdbTransaction) -> Completion {
        let slot = Rc::new(Slot::new());
        let on_complete = {
            let slot = slot.clone();
            Closure::<dyn FnMut()>::new(move || slot.finish(Ok(())))
        };
        let on_abort = {
            let (tx, slot) = (tx.clone(), slot.clone());
            Closure::<dyn FnMut()>::new(move || {
                let e = match tx.error() {
                    Some(e) => Error::Backend(js::describe(&e)),
                    None => Error::Backend("transaction aborted".to_owned()),
                };
                slot.finish(Err(e));
            })
        };
        tx.set_oncomplete(Some(on_complete.as_ref().unchecked_ref()));
        tx.set_onabort(Some(on_abort.as_ref().unchecked_ref()));
        Completion {
            tx: tx.clone(),
            slot,
            _on_complete: on_complete,
            _on_abort: on_abort,
        }
    }
}

impl Future for Completion {
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.slot.poll(cx)
    }
}

impl Drop for Completion {
    fn drop(&mut self) {
        self.tx.set_oncomplete(None);
        self.tx.set_onabort(None);
    }
}

/// Opens `name` at [`VERSION`].
///
/// A new database gets the store when `create` is set. Otherwise its
/// creation is aborted, which leaves no database behind, and the open
/// fails with [`Error::NoTable`]; so does an existing database without the
/// store. Resolves to the connection and whether the database is new.
///
/// The open runs to completion even if the future is dropped: the
/// request's handler owns its state until the request ends, aborts the
/// creation of a database nobody awaits, and closes its connection.
pub(crate) fn open(factory: &IdbFactory, name: &str, create: bool) -> Result<Open, Error> {
    let req = factory
        .open_with_u32(name, VERSION)
        .map_err(|e| error(&e))?;
    let op = Rc::new(OpenOp {
        req,
        create,
        upgraded: Cell::new(false),
        abandoned: Cell::new(false),
        slot: Slot::new(),
        handler: RefCell::new(None),
    });
    let handler = {
        let op = op.clone();
        EventHandler::new(move |e: Event| op.handle(&e))
    };
    let f: &js_sys::Function = handler.as_ref().unchecked_ref();
    op.req.set_onupgradeneeded(Some(f));
    op.req.set_onsuccess(Some(f));
    op.req.set_onerror(Some(f));
    // A cycle through the handler, broken when the request ends.
    op.handler.replace(Some(handler));
    Ok(Open { op })
}

/// See [`open`].
pub(crate) struct Open {
    op: Rc<OpenOp>,
}

struct OpenOp {
    req: IdbOpenDbRequest,
    create: bool,
    upgraded: Cell<bool>,
    /// The [`Open`] was dropped before the request ended.
    abandoned: Cell<bool>,
    slot: Slot<(IdbDatabase, bool)>,
    handler: RefCell<Option<EventHandler>>,
}

type EventHandler = Closure<dyn FnMut(Event)>;

impl OpenOp {
    fn handle(&self, event: &Event) {
        match event.type_().as_str() {
            "upgradeneeded" => self.upgrade(),
            "success" => self.succeed(),
            _ => self.fail(),
        }
    }

    /// Runs only for a new database: every existing one is at [`VERSION`].
    fn upgrade(&self) {
        self.upgraded.set(true);
        let created = self.create
            && !self.abandoned.get()
            && self.req.result().ok().is_some_and(|db| {
                db.unchecked_into::<IdbDatabase>()
                    .create_object_store(STORE)
                    .is_ok()
            });
        if !created && let Some(tx) = self.req.transaction() {
            let _ = tx.abort();
        }
    }

    fn succeed(&self) {
        let out = match self.req.result() {
            Ok(db) => {
                let db: IdbDatabase = db.unchecked_into();
                if db.object_store_names().contains(STORE) {
                    Ok((db, self.upgraded.get()))
                } else {
                    db.close();
                    Err(Error::NoTable)
                }
            }
            Err(e) => Err(error(&e)),
        };
        self.end(out);
    }

    fn fail(&self) {
        let out = if self.upgraded.get() && !self.create {
            Err(Error::NoTable)
        } else {
            Err(request_error(&self.req))
        };
        self.end(out);
    }

    fn end(&self, out: Result<(IdbDatabase, bool), Error>) {
        self.req.set_onupgradeneeded(None);
        self.req.set_onsuccess(None);
        self.req.set_onerror(None);
        // The handler is running: wasm-bindgen frees it once it returns.
        let handler = self.handler.take();
        if self.abandoned.get() {
            if let Ok((db, _)) = out {
                db.close();
            }
        } else {
            self.slot.finish(out);
        }
        drop(handler);
    }
}

impl Future for Open {
    type Output = Result<(IdbDatabase, bool), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.op.slot.poll(cx)
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        self.op.abandoned.set(true);
        if let Some(Ok((db, _))) = self.op.slot.out.take() {
            db.close();
        }
    }
}

/// Deletes database `name`. `on_blocked` runs while open connections hold
/// the deletion back.
pub(crate) fn delete(
    factory: &IdbFactory,
    name: &str,
    on_blocked: impl FnMut() + 'static,
) -> Result<Request<()>, Error> {
    let req = factory.delete_database(name).map_err(|e| error(&e))?;
    Ok(Request::new(req.unchecked_into(), |_| Some(Ok(()))).on_blocked(on_blocked))
}
