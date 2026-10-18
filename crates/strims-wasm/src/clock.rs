// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Time in the worker: the clock, sleeps and the macrotask yield.
//!
//! - [`now`] is `performance.timeOrigin + performance.now()`, read in
//!   microseconds and truncated to [`Time::PRECISION`]. It never goes
//!   backwards for the life of the worker.
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
use web_sys::{MessageChannel, MessagePort, Performance};

use crate::js;

/// The longest delay `setTimeout` takes; longer ones fire at once.
const MAX_TIMEOUT_MS: i64 = i32::MAX as i64;

thread_local! {
    /// The global's `performance`, and its time origin in microseconds,
    /// which is fixed for the life of the global.
    static PERFORMANCE: Option<(Performance, i64)> = js::scope()
        .performance()
        .map(|p| {
            let origin = micros(p.time_origin());
            (p, origin)
        });

    static YIELDER: Option<Yielder> = Yielder::new();
}

/// Milliseconds as whole microseconds. `as` saturates, and NaN is 0.
fn micros(ms: f64) -> i64 {
    (ms * 1_000.0) as i64
}

/// The current time, at [`Time::PRECISION`].
pub fn now() -> Time {
    // The worker's clock, and the one place the crate reads the time.
    // Every worker and window has `performance`; `Date.now()` stands in
    // where a global lacks it.
    let micros = PERFORMANCE.with(|p| match p {
        Some((performance, origin)) => origin.saturating_add(micros(performance.now())),
        None => micros(js_sys::Date::now()),
    });
    Time::from_unix_nanos(micros.saturating_mul(1_000)).truncate(Time::PRECISION)
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
