// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Time in the worker: the clock, sleeps and the macrotask yield.
//!
//! - [`now`] is the wall clock, `Date.now()`, at [`Time::PRECISION`].
//!   Certificates and record timestamps are wall time, and a monotonic
//!   clock can stop while the machine sleeps, so the node would fall
//!   behind wall time for the rest of its life. Like any wall clock, it
//!   can step backwards.
//! - Reading the clock costs microseconds in some browsers, and the node
//!   reads it for every input, so [`now`] reads it once per turn of the
//!   event loop. The first read of a turn queues a microtask that forgets
//!   the value; every read before it runs returns the same time. That
//!   covers the rest of the turn's callback and the tasks the executor
//!   polls in the microtask it was already running, so time moves on
//!   between turns and never within one, and a long turn ends with the
//!   time it started at.
//! - A [`Sleep`] waits on `setTimeout`. A timer can fire before [`now`]
//!   reaches its deadline: delays are clamped to 2^31−1 ms, and browsers
//!   round timers and the clock differently. A sleep woken early arms a
//!   timer for the rest, so it never resolves before its deadline.
//! - [`yield_now`] lets the events queued before it run, through a
//!   `MessageChannel` message (see the [crate docs](crate)).
//!
//! [`WasmClock`] is the services' [`Timer`]. A driver loop uses the free
//! functions [`now`], [`new_sleep`], [`reset_sleep`] and [`yield_now`].

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use strims_base::time::{Duration, Time};
use strims_rpc::Timer;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageChannel, MessagePort};

use crate::js;

/// The longest delay `setTimeout` takes; longer ones fire at once.
const MAX_TIMEOUT_MS: i64 = i32::MAX as i64;

thread_local! {
    static YIELDER: Option<Yielder> = Yielder::new();

    static TURN: Turn = Turn::new();
}

/// The time read in this turn of the event loop, until the microtask that
/// forgets it runs.
struct Turn {
    now: Cell<Option<Time>>,
    forget: Closure<dyn FnMut()>,
}

impl Turn {
    fn new() -> Turn {
        Turn {
            now: Cell::new(None),
            forget: Closure::new(|| TURN.with(|t| t.now.set(None))),
        }
    }
}

/// The current wall time, at [`Time::PRECISION`], as of the first read in
/// this turn of the event loop.
pub fn now() -> Time {
    TURN.with(|t| {
        if let Some(now) = t.now.get() {
            return now;
        }
        // The worker's clock, and the one place the node reads the time.
        // `as` saturates, and NaN is 0.
        let now = Time::from_unix_millis(js_sys::Date::now() as i64);
        t.now.set(Some(now));
        js::queue_microtask(t.forget.as_ref().unchecked_ref());
        now
    })
}

/// A sleep that is already due: the driver loop's timer before its first
/// [`reset_sleep`].
pub fn new_sleep() -> Sleep {
    Sleep::until(Time::NIL)
}

/// Moves `sleep`'s deadline to `deadline`; see [`Sleep::reset`].
pub fn reset_sleep(sleep: Pin<&mut Sleep>, deadline: Time) {
    sleep.get_mut().reset(deadline);
}

/// Resolves once the events queued before its first poll have run.
pub fn yield_now() -> YieldNow {
    YieldNow::default()
}

/// The worker's [`Timer`]: [`now`] and [`Sleep`].
#[derive(Clone, Copy, Debug, Default)]
pub struct WasmClock;

impl Timer for WasmClock {
    type Sleep = Sleep;

    fn now(&self) -> Time {
        now()
    }

    fn sleep_until(&self, deadline: Time) -> Sleep {
        Sleep::until(deadline)
    }
}

/// Resolves once [`now`] has reached its deadline, and again at every
/// later poll until the deadline moves. Dropping it clears its timer.
#[derive(Debug)]
pub struct Sleep {
    deadline: Time,
    timer: Option<Timeout>,
}

impl Sleep {
    /// A sleep until `deadline`.
    pub fn until(deadline: Time) -> Sleep {
        Sleep {
            deadline,
            timer: None,
        }
    }

    /// The time this sleep resolves at.
    pub fn deadline(&self) -> Time {
        self.deadline
    }

    /// Moves the deadline to `deadline`. The task must poll the sleep
    /// again for the new deadline to wake it.
    pub fn reset(&mut self, deadline: Time) {
        if deadline != self.deadline {
            self.deadline = deadline;
            self.timer = None;
        }
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = self.get_mut();
        let now = now();
        if now >= this.deadline {
            this.timer = None;
            return Poll::Ready(());
        }
        match &this.timer {
            Some(timer) if !timer.wake.fired.get() => timer.wake.register(cx.waker()),
            _ => this.timer = Some(Timeout::arm(this.deadline - now, cx.waker())),
        }
        Poll::Pending
    }
}

/// One `setTimeout`, cleared on drop unless it has fired.
#[derive(Debug)]
struct Timeout {
    id: JsValue,
    wake: Rc<TimerWake>,
    _callback: Closure<dyn FnMut()>,
}

#[derive(Debug, Default)]
struct TimerWake {
    fired: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl TimerWake {
    fn register(&self, waker: &Waker) {
        let current = self
            .waker
            .borrow()
            .as_ref()
            .is_some_and(|w| w.will_wake(waker));
        if !current {
            let old = self.waker.borrow_mut().replace(waker.clone());
            drop(old);
        }
    }

    fn fire(&self) {
        self.fired.set(true);
        let waker = self.waker.borrow_mut().take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl Timeout {
    /// A timer that wakes `waker` in at least `delay` (positive), rounded
    /// up to whole milliseconds.
    fn arm(delay: Duration, waker: &Waker) -> Timeout {
        let nanos = delay.as_nanos();
        let ms = (nanos / 1_000_000 + i64::from(nanos % 1_000_000 != 0)).clamp(1, MAX_TIMEOUT_MS);
        let wake = Rc::new(TimerWake::default());
        wake.register(waker);
        let callback = {
            let wake = wake.clone();
            Closure::<dyn FnMut()>::new(move || wake.fire())
        };
        let id = js::set_timeout(
            callback.as_ref().unchecked_ref(),
            i32::try_from(ms).unwrap_or(i32::MAX),
        );
        Timeout {
            id,
            wake,
            _callback: callback,
        }
    }
}

impl Drop for Timeout {
    fn drop(&mut self) {
        if !self.wake.fired.get() {
            js::clear_timeout(&self.id);
        }
    }
}

/// The reused channel [`YieldNow`] posts through. Messages arrive in the
/// order they were posted, so the n-th message releases the n-th yield.
struct Yielder {
    send: MessagePort,
    _recv: MessagePort,
    state: Rc<YieldState>,
    _on_message: Closure<dyn FnMut()>,
}

#[derive(Default)]
struct YieldState {
    posted: Cell<u64>,
    delivered: Cell<u64>,
    /// Pending yields by ticket. One entry per live [`YieldNow`].
    waiters: RefCell<BTreeMap<u64, Waker>>,
}

impl Yielder {
    fn new() -> Option<Yielder> {
        let channel = MessageChannel::new().ok()?;
        let state = Rc::new(YieldState::default());
        let on_message = {
            let state = state.clone();
            Closure::<dyn FnMut()>::new(move || state.deliver())
        };
        let recv = channel.port1();
        // Setting `onmessage` starts the port.
        recv.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        Some(Yielder {
            send: channel.port2(),
            _recv: recv,
            state,
            _on_message: on_message,
        })
    }
}

impl YieldState {
    fn deliver(&self) {
        let delivered = self.delivered.get().saturating_add(1);
        self.delivered.set(delivered);
        let ready = {
            let mut waiters = self.waiters.borrow_mut();
            let later = waiters.split_off(&delivered.saturating_add(1));
            std::mem::replace(&mut *waiters, later)
        };
        for waker in ready.into_values() {
            waker.wake();
        }
    }
}

/// See [`yield_now`]. Without a `MessageChannel` it resolves at once.
#[derive(Debug, Default)]
pub struct YieldNow {
    state: YieldStep,
}

#[derive(Debug, Default, Clone, Copy)]
enum YieldStep {
    #[default]
    Start,
    Waiting(u64),
    Done,
}

impl Future for YieldNow {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = self.get_mut();
        let step = YIELDER.with(|y| {
            let Some(y) = y else {
                return YieldStep::Done;
            };
            let s = &y.state;
            match this.state {
                YieldStep::Start => {
                    if y.send.post_message(&JsValue::UNDEFINED).is_err() {
                        return YieldStep::Done;
                    }
                    let ticket = s.posted.get().saturating_add(1);
                    s.posted.set(ticket);
                    let waker = cx.waker().clone();
                    s.waiters.borrow_mut().insert(ticket, waker);
                    YieldStep::Waiting(ticket)
                }
                YieldStep::Waiting(ticket) if s.delivered.get() >= ticket => YieldStep::Done,
                YieldStep::Waiting(ticket) => {
                    let current = s
                        .waiters
                        .borrow()
                        .get(&ticket)
                        .is_some_and(|w| w.will_wake(cx.waker()));
                    if !current {
                        let waker = cx.waker().clone();
                        let old = s.waiters.borrow_mut().insert(ticket, waker);
                        drop(old);
                    }
                    YieldStep::Waiting(ticket)
                }
                YieldStep::Done => YieldStep::Done,
            }
        });
        this.state = step;
        match step {
            YieldStep::Done => Poll::Ready(()),
            _ => Poll::Pending,
        }
    }
}

impl Drop for YieldNow {
    fn drop(&mut self) {
        if let YieldStep::Waiting(ticket) = self.state {
            let waker = YIELDER
                .try_with(|y| y.as_ref()?.state.waiters.borrow_mut().remove(&ticket))
                .ok()
                .flatten();
            drop(waker);
        }
    }
}
