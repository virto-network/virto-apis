#![no_std]
extern crate alloc;

#[cfg(feature = "query")]
pub mod query;

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};
use core::{cell::RefCell, future::Future, pin::Pin, str::FromStr};
pub use erased_serde::Serialize;

pub struct Context<S, E> {
    state: Rc<S>,
    meta: BTreeMap<String, String>,
    events: RefCell<Vec<E>>,
}

impl<S, E> Context<S, E> {
    pub fn new(
        state: Rc<S>,
        meta: impl IntoIterator<Item = (impl ToString, impl ToString)>,
    ) -> Self {
        Context {
            state,
            meta: meta
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            events: RefCell::new(Vec::new()),
        }
    }

    pub fn state(&self) -> Rc<S> {
        self.state.clone()
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.meta.get(key).map(|v| v.as_ref())
    }

    /// get a metadata value inferring it from the return type
    pub fn get<U: FromStr + MetaKey>(&self) -> Option<U> {
        self.get_str(U::KEY).map(|k| k.parse().ok()).flatten()
    }

    /// Queue an event for later processing
    pub fn put_event(&self, event: impl Into<E>) {
        self.events.borrow_mut().push(event.into());
    }

    /// Consume pending events. Since `Context` is passed to message handlers as an immutable borrow
    /// it prevents users from calling it and can only be used by the outer code that created the `Context`
    /// usually at the end of the handler's execution.
    pub fn events(&mut self) -> impl Iterator<Item = E> + '_ {
        self.events.get_mut().drain(..)
    }
}

pub trait MetaKey {
    const KEY: &'static str;
}

type WaitResult<'a, E> = Pin<Box<dyn Future<Output = Result<Box<dyn Serialize>, E>> + 'a>>;
type AsyncTask<'a, E> = Pin<Box<dyn Future<Output = Result<(), E>> + 'a>>;

pub enum End<'a, E> {
    Async(AsyncTask<'a, E>),
    WaitResult(WaitResult<'a, E>),
}

pub type EndResult<'a, E> = Result<End<'a, E>, E>;
