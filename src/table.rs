use std::{marker::PhantomData, num::NonZeroUsize, ops::Deref};

#[repr(transparent)]
pub struct Tab<I, V> {
    _phantom: PhantomData<I>,
    items: [Option<V>],
}

impl<I: Id, V> Tab<I, V> {
    pub fn get(&self, id: I) -> Option<&V> {
        match self.items.get(id.idx()) {
            Some(Some(value)) => Some(value),
            Some(None) | None => None,
        }
    }

    pub fn get_mut(&mut self, id: I) -> Option<&mut V> {
        match self.items.get_mut(id.idx()) {
            Some(Some(value)) => Some(value),
            Some(None) | None => None,
        }
    }
}

pub struct Table<I, V> {
    _phantom: PhantomData<I>,
    items: Vec<Option<V>>,
}

impl<I: Id, V> Table<I, V> {
    pub const fn new() -> Table<I, V> {
        Table {
            _phantom: PhantomData,
            items: Vec::new(),
        }
    }

    pub fn insert(&mut self, id: I, value: V) {
        let idx = id.idx();
        if idx >= self.items.len() {
            self.items.resize_with(idx + 1, || None);
        }
        self.items[idx] = Some(value);
    }
}

impl<I, V> Table<I, V> {
    pub fn into_boxed_tab(self) -> Box<Tab<I, V>> {
        let ptr = Box::into_raw(self.items.into_boxed_slice());
        unsafe { Box::from_raw(ptr as *mut Tab<I, V>) }
    }
}

impl<I, V> Deref for Table<I, V> {
    type Target = Tab<I, V>;

    fn deref(&self) -> &Tab<I, V> {
        let slice = &*self.items;
        unsafe { &*(slice as *const [Option<V>] as *const Tab<I, V>) }
    }
}

pub trait Id: Clone + Copy + Into<NonZeroUsize> {}

trait IdExt {
    fn idx(self) -> usize;
}

impl<I: Id> IdExt for I {
    fn idx(self) -> usize {
        self.into().get() - 1
    }
}
