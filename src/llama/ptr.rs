//! Thread-safe wrapper for raw FFI pointers.
//!
//! # Safety
//!
//! The user must ensure that the wrapped pointer is only accessed from a single thread
//! at a time, or that proper synchronization is used externally.

use std::sync::Arc;
use std::sync::Mutex;

/// Wraps a raw pointer with Send + Sync implementation.
/// # Safety
/// The caller must ensure exclusive access to the underlying pointer.
unsafe impl<T: Send> Send for SendPtr<T> {}
unsafe impl<T: Send> Sync for SyncPtr<T> {}

/// A pointer type that claims Send + Sync for use in FFI wrappers.
/// # Safety
/// The caller must ensure proper synchronization when accessing the underlying pointer.
pub struct SendPtr<T>(pub T);
pub struct SyncPtr<T>(pub T);

impl<T> SendPtr<T> {
    pub fn new(ptr: T) -> Self {
        Self(ptr)
    }
    pub fn get(&self) -> &T {
        &self.0
    }
}

impl<T> SyncPtr<T> {
    pub fn new(ptr: T) -> Self {
        Self(ptr)
    }
    pub fn get(&self) -> &T {
        &self.0
    }
}

pub type ModelPtr = SendPtr<*mut libc::c_void>;
pub type ContextPtr = SyncPtr<*mut libc::c_void>;