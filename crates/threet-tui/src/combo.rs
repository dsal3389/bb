use std::collections::BTreeMap;
use std::pin::Pin;

use crate::app::Context;
use crate::event::Key;
use crate::event::KeyCode;
use crate::views::View;

// lifetime: the returned future is bounded to the `Context` lifetime, it is possible because we don't
// hand this callback over to `tokio::spawn` which would have required us to have `'static` lifetime
pub type ComboCallback = fn(Context<'_>) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;

// set the max combo depth for keys
pub const MAX_COMBO_DEPTH: usize = 8;

/// a node that represent a key in a path, or an action
#[derive(Default)]
struct Combo {
    entries: BTreeMap<Key, Combo>,
    callback: Option<ComboCallback>,
}

impl Combo {
    fn add<I>(&mut self, mut combo: I, callback: ComboCallback)
    where
        I: Iterator<Item = Key>,
    {
        // if there is a next key in the combo iterator
        // it means the current node should hold a child
        // and pass the combo to the child, if we are at the end of the combo (`None`)
        // it means the current node should hold the callback
        match combo.next() {
            Some(key) => self.entries.entry(key).or_default().add(combo, callback),
            None => self.callback = Some(callback),
        };
    }

    /// get the callback based on the given iterator next value
    fn get<I>(&self, mut keys: I) -> Option<&ComboCallback>
    where
        I: Iterator<Item = Key>,
    {
        match keys.next() {
            Some(key) => self.entries.get(&key).and_then(|node| node.get(keys)),
            None => self.callback.as_ref(),
        }
    }
}

#[repr(transparent)]
pub struct ComboRegister {
    root: Combo,
}

impl ComboRegister {
    pub fn new() -> Self {
        Self {
            root: Combo::default(),
        }
    }

    /// add a combo to the combo register
    #[inline(always)]
    pub fn add<I>(&mut self, keys: I, callback: ComboCallback)
    where
        I: IntoIterator<Item = Key>,
    {
        self.root.add(keys.into_iter(), callback)
    }

    #[inline(always)]
    pub fn get<I>(&self, keys: I) -> Option<&ComboCallback>
    where
        I: IntoIterator,
        I::Item: AsRef<Key>,
    {
        self.root
            .get(keys.into_iter().map(|key| key.as_ref().clone()))
    }
}

pub struct ComboRecorder(Vec<Key>);

impl ComboRecorder {
    pub fn new() -> ComboRecorder {
        ComboRecorder(Vec::with_capacity(MAX_COMBO_DEPTH))
    }

    /// extends the recorder inner buffer with the iterator keys
    pub fn extend<I>(&mut self, keys: I)
    where
        I: IntoIterator<Item = Key>,
    {
        for key in keys {
            if key.keycode == KeyCode::Esc {
                self.0.clear();
            } else if self.0.len() < MAX_COMBO_DEPTH {
                // prevent pushing keys to the vector, this will also not allow
                // for more allocations from the vector
                self.0.push(key);
            }
        }
    }

    /// clear all the pressed keys in the recorder
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl AsRef<[Key]> for ComboRecorder {
    fn as_ref(&self) -> &[Key] {
        self.0.as_slice()
    }
}
