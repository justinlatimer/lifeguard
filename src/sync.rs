#![allow(dead_code)]

extern crate parking_lot;

use std::sync::Arc;
use self::parking_lot::RwLock;
use std::fmt;
use std::ops::{Drop, Deref, DerefMut};
use std::convert::{AsRef, AsMut};
use std::borrow::Borrow;
use {Recycleable, InitializeWith};

/// A smartpointer which uses a shared reference (`&`) to know
/// when to move its wrapped value back to the `Pool` that
/// issued it.
pub struct Recycled<'a, T: 'a>
    where T: Recycleable + Sync + Send
{
    value: RecycledInner<&'a RwLock<CappedCollection<T>>, T>,
}

/// A smartpointer which uses reference counting (`Arc`) to know
/// when to move its wrapped value back to the `Pool` that
/// issued it.
pub struct ArcRecycled<T>
    where T: Recycleable + Sync + Send
{
    value: RecycledInner<Arc<RwLock<CappedCollection<T>>>, T>,
}

macro_rules! impl_recycled {
  ($name: ident, $typ: ty, $pool: ty) => {
  impl <'a, T> AsRef<T> for $typ where T : Recycleable + Sync + Send {
     /// Gets a shared reference to the value wrapped by the smartpointer.
     fn as_ref(&self) -> &T {
      self.value.as_ref()
    }
  }

  impl <'a, T> AsMut<T> for $typ where T : Recycleable + Sync + Send {
     /// Gets a mutable reference to the value wrapped by the smartpointer.
     fn as_mut(&mut self) -> &mut T {
      self.value.as_mut()
    }
  }

  impl <'a, T> fmt::Debug for $typ where T : fmt::Debug + Recycleable + Sync + Send {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      self.value.fmt(f)
    }
  }

  impl <'a, T> fmt::Display for $typ where T : fmt::Display + Recycleable + Sync + Send {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      self.value.fmt(f)
    }
  }

  impl <'a, T> Deref for $typ where T : Recycleable + Sync + Send {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
      self.as_ref()
    }
  }

  impl <'a, T> DerefMut for $typ where T : Recycleable + Sync + Send {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
      self.as_mut()
    }
  }

  impl <'a, T> $typ where T: Recycleable + Sync + Send {
    fn new(pool: $pool, value: T) -> $typ {
      $name { value: RecycledInner::new(pool, value) }
    }

    #[inline]
    fn new_from<A>(pool: $pool, value: T, source: A) -> $typ where T : InitializeWith<A> {
      $name { value: RecycledInner::new_from(pool, value, source) }
    }

    #[inline]
    /// Disassociates the value from the `Pool` that issued it. This
    /// destroys the smartpointer and returns the previously wrapped value.
    pub fn detach(self) -> T {
      self.value.detach()
    }
  }
}
}

impl_recycled!{ ArcRecycled, ArcRecycled<T>, Arc<RwLock<CappedCollection<T>>> }
impl_recycled!{ Recycled, Recycled<'a, T>, &'a RwLock<CappedCollection<T>> }

struct RecycledInner<P, T>
    where P: Borrow<RwLock<CappedCollection<T>>>,
          T: Recycleable + Sync + Send
{
    value: Option<T>,
    pool: P,
}

impl<P, T> Drop for RecycledInner<P, T>
    where P: Borrow<RwLock<CappedCollection<T>>>,
          T: Recycleable + Sync + Send
{
    #[inline]
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            self.pool.borrow().write().insert_or_drop(value);
        }
    }
}

impl<P, T> AsRef<T> for RecycledInner<P, T>
    where P: Borrow<RwLock<CappedCollection<T>>>,
          T: Recycleable + Sync + Send
{
    fn as_ref(&self) -> &T {
        match self.value.as_ref() {
            Some(v) => v,
            None => panic!("Recycled<T> smartpointer missing its value."),
        }
    }
}

impl<P, T> AsMut<T> for RecycledInner<P, T>
    where P: Borrow<RwLock<CappedCollection<T>>>,
          T: Recycleable + Sync + Send
{
    fn as_mut(&mut self) -> &mut T {
        match self.value.as_mut() {
            Some(v) => v,
            None => panic!("Recycled<T> smartpointer missing its value."),
        }
    }
}

impl<P, T> fmt::Debug for RecycledInner<P, T>
    where P: Borrow<RwLock<CappedCollection<T>>>,
          T: fmt::Debug + Recycleable + Sync + Send
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.value {
            Some(ref s) => s.fmt(f),
            None => write!(f, "Empty Recycled<T>"),
        }
    }
}

impl<P, T> fmt::Display for RecycledInner<P, T>
    where P: Borrow<RwLock<CappedCollection<T>>>,
          T: fmt::Display + Recycleable + Sync + Send
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.value {
            Some(ref s) => s.fmt(f),
            None => write!(f, "Empty Recycled<T>"),
        }
    }
}

impl<P, T> Deref for RecycledInner<P, T>
    where P: Borrow<RwLock<CappedCollection<T>>>,
          T: Recycleable + Sync + Send
{
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<P, T> DerefMut for RecycledInner<P, T>
    where P: Borrow<RwLock<CappedCollection<T>>>,
          T: Recycleable + Sync + Send
{
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}

impl<P, T> RecycledInner<P, T>
    where P: Borrow<RwLock<CappedCollection<T>>>,
          T: Recycleable + Sync + Send
{
    #[inline]
    fn new(pool: P, value: T) -> RecycledInner<P, T> {
        RecycledInner {
            value: Some(value),
            pool: pool,
        }
    }

    #[inline]
    fn new_from<A>(pool: P, mut value: T, source: A) -> RecycledInner<P, T>
        where T: InitializeWith<A>
    {
        value.initialize_with(source);
        RecycledInner {
            value: Some(value),
            pool: pool,
        }
    }

    #[inline]
    fn detach(mut self) -> T {
        let value = self.value.take().unwrap();
        drop(self);
        value
    }
}

struct CappedCollection<T>
    where T: Recycleable + Sync + Send
{
    values: Vec<T>,
    cap: usize,
    supplier: Box<Supply<Output = T>>,
}

impl<T> CappedCollection<T>
    where T: Recycleable + Sync + Send
{
    #[inline]
    pub fn new(mut supplier: Box<Supply<Output = T>>,
               starting_size: usize,
               max_size: usize)
               -> CappedCollection<T> {
        use std::cmp;
        let starting_size = cmp::min(starting_size, max_size);
        let values: Vec<T> = (0..starting_size)
            .map(|_| supplier.get())
            .collect();
        CappedCollection {
            values: values,
            cap: max_size,
            supplier: supplier,
        }
    }

    #[inline]
    pub fn insert_or_drop(&mut self, mut value: T) {
        if self.is_full() {
            drop(value);
        } else {
            value.reset();
            self.values.push(value);
        }
    }

    #[inline]
    pub fn remove(&mut self) -> Option<T> {
        self.values.pop()
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.values.len() >= self.cap
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[inline]
    pub fn cap(&self) -> usize {
        self.cap
    }
}

/// Provides a method which will produce new instances of a type
pub trait Supply
    where Self: Sync + Send
{
    type Output: Recycleable + Sync + Send;

    fn get(&mut self) -> Self::Output;
}

impl<F, T: Sync + Send> Supply for F
    where F: FnMut() -> T + Send + Sync,
          T: Recycleable
{
    type Output = T;
    fn get(&mut self) -> T {
        self()
    }
}

/// A collection of values that can be reused without requiring new allocations.
///
/// `Pool` issues each value wrapped in a smartpointer. When the smartpointer goes out of
/// scope, the wrapped value is automatically returned to the pool.
pub struct Pool<T>
    where T: Recycleable + Sync + Send
{
    values: Arc<RwLock<CappedCollection<T>>>,
}

impl<T> Pool<T>
    where T: Recycleable + Sync + Send
{
    /// Creates a pool with `size` elements of type `T` allocated.
    #[inline]
    pub fn with_size(size: usize) -> Pool<T> {
        use std::usize;
        Pool::with_size_and_max(size, usize::max_value())
    }

    /// Creates a pool with `size` elements of type `T` allocated
    /// and sets a maximum pool size of `max_size`. Values being
    /// added to the pool via `Pool::attach` or being returned to
    /// the pool upon dropping will instead be discarded if the pool
    /// is full.
    #[inline]
    pub fn with_size_and_max(starting_size: usize, max_size: usize) -> Pool<T> {
        let supplier = Box::new(|| T::new());
        let values: CappedCollection<T> = CappedCollection::new(supplier, starting_size, max_size);
        Pool { values: Arc::new(RwLock::new(values)) }
    }

    /// Returns the number of values remaining in the pool.
    #[inline]
    pub fn size(&self) -> usize {
        (*self.values).read().len()
    }

    /// Returns the maximum number of values the pool can hold.
    #[inline]
    pub fn max_size(&self) -> usize {
        (*self.values).read().cap()
    }

    /// Removes a value from the pool and returns it wrapped in
    /// a `Recycled smartpointer. If the pool is empty when the
    /// method is called, a new value will be allocated.
    #[inline]
    pub fn new(&self) -> Recycled<T> {
        let t = self.detached();
        Recycled { value: RecycledInner::new(&*self.values, t) }
    }

    /// Removes a value from the pool, initializes it using the provided
    /// source value, and returns it wrapped in a `Recycled` smartpointer.
    /// If the pool is empty when the method is called, a new value will be
    /// allocated.
    #[inline(always)]
    pub fn new_from<A>(&self, source: A) -> Recycled<T>
        where T: InitializeWith<A>
    {
        let t = self.detached();
        Recycled { value: RecycledInner::new_from(&*self.values, t, source) }
    }

    /// Associates the provided value with the pool by wrapping it in a
    /// `Recycled` smartpointer.
    #[inline]
    pub fn attach(&self, value: T) -> Recycled<T> {
        Recycled { value: RecycledInner::new(&*self.values, value) }
    }

    /// Removes a value from the pool and returns it without wrapping it in
    /// a smartpointer. When the value goes out of scope it will not be
    /// returned to the pool.
    #[inline]
    pub fn detached(&self) -> T {
        let mut collection = self.values.write();
        let maybe_value = collection.remove();
        match maybe_value {
            Some(v) => v,
            None => collection.supplier.get(),
        }
    }

    /// Removes a value from the pool and returns it wrapped in
    /// an `ArcRecycled` smartpointer. If the pool is empty when the
    /// method is called, a new value will be allocated.
    #[inline]
    pub fn new_arc(&self) -> ArcRecycled<T> {
        let t = self.detached();
        let pool_reference = self.values.clone();
        ArcRecycled { value: RecycledInner::new(pool_reference, t) }
    }

    /// Removes a value from the pool, initializes it using the provided
    /// source value, and returns it wrapped in an `ArcRecycled` smartpointer.
    /// If the pool is empty when the method is called, a new value will be
    /// allocated.
    #[inline(always)]
    pub fn new_rc_from<A>(&self, source: A) -> ArcRecycled<T>
        where T: InitializeWith<A>
    {
        let t = self.detached();
        let pool_reference = self.values.clone();
        ArcRecycled { value: RecycledInner::new_from(pool_reference, t, source) }
    }

    /// Associates the provided value with the pool by wrapping it in an
    /// `ArcRecycled` smartpointer.
    #[inline]
    pub fn attach_rc(&self, value: T) -> ArcRecycled<T> {
        let pool_reference = self.values.clone();
        ArcRecycled { value: RecycledInner::new(pool_reference, value) }
    }
}

/// Produces a `PoolBuilder` instance
///
/// # Example
///
/// ```
/// extern crate lifeguard;
/// use lifeguard::*;
///
/// fn main() {
///   let mut pool: Pool<String> = pool()
///     .with(StartingSize(128))
///     .with(MaxSize(4096))
///     .with(Supplier(|| String::with_capacity(1024)))
///     .build();
/// }
/// ```
pub fn pool<T>() -> PoolBuilder<T>
    where T: Recycleable
{
    use std::usize;
    PoolBuilder {
        starting_size: 16,
        max_size: usize::max_value(),
        supplier: None,
    }
}

/// Used to define settings for and ultimately create a `Pool`.
pub struct PoolBuilder<T>
    where T: Recycleable
{
    pub starting_size: usize,
    pub max_size: usize,
    pub supplier: Option<Box<Supply<Output = T>>>,
}

impl<T> PoolBuilder<T>
    where T: Recycleable + Sync + Send
{
    pub fn with<U>(self, option_setter: U) -> PoolBuilder<T>
        where U: OptionSetter<PoolBuilder<T>>
    {
        option_setter.set_option(self)
    }

    pub fn build(self) -> Pool<T>
        where T: Recycleable
    {
        let supplier = self.supplier.unwrap_or_else(|| Box::new(|| T::new()));
        let values: CappedCollection<T> =
            CappedCollection::new(supplier, self.starting_size, self.max_size);
        Pool { values: Arc::new(RwLock::new(values)) }
    }
}

pub mod settings {
    use ::Recycleable;
    use super::{PoolBuilder, Supply};
    /// Implementing this trait allows a struct to act as a configuration
    /// parameter in the builder API.
    pub trait OptionSetter<T> {
        fn set_option(self, T) -> T;
    }

    /// Specifies how many values should be requested from the Supplier at
    /// initialization time. These values will be available for immediate use.
    pub struct StartingSize(pub usize);
    /// Specifies the largest number of values the `Pool` will hold before it
    /// will begin to drop values being returned to it.
    pub struct MaxSize(pub usize);
    /// Specifies a value implementing `Supply<Output=T>` that will be used to allocate
    /// new values. If unspecified, `T::new()` will be invoked.
    pub struct Supplier<S>(pub S) where S: Supply;

    impl<T> OptionSetter<PoolBuilder<T>> for StartingSize
        where T: Recycleable
    {
        fn set_option(self, mut builder: PoolBuilder<T>) -> PoolBuilder<T> {
            let StartingSize(size) = self;
            builder.starting_size = size;
            builder
        }
    }

    impl<T> OptionSetter<PoolBuilder<T>> for MaxSize
        where T: Recycleable
    {
        fn set_option(self, mut builder: PoolBuilder<T>) -> PoolBuilder<T> {
            let MaxSize(size) = self;
            builder.max_size = size;
            builder
        }
    }

    impl<T, S> OptionSetter<PoolBuilder<T>> for Supplier<S>
        where S: Supply<Output = T> + 'static,
              T: Recycleable + Send + Sync
    {
        fn set_option(self, mut builder: PoolBuilder<T>) -> PoolBuilder<T> {
            let Supplier(supplier) = self;
            builder.supplier = Some(Box::new(supplier) as Box<Supply<Output = T>>);
            builder
        }
    }
}

pub use sync::settings::{OptionSetter, StartingSize, MaxSize, Supplier};
