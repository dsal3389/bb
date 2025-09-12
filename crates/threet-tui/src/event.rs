use core::str;
use std::time::Duration;

use threet_storage::models::User;

use crate::notifications::Notification;

#[derive(Debug, Clone, Hash, Eq, Ord, PartialOrd, PartialEq)]
pub enum KeyCode {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Backspace,
    Enter,
    Space,
    Tab,
    Esc,
    Char(char),
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Modifier(u32);

impl Modifier {
    const NONE: Modifier = Modifier(0x0);
    const SHIFT: Modifier = Modifier(0x1);
    const CTRL: Modifier = Modifier(0x2);

    #[inline(always)]
    pub fn contains(&self, modifier: Modifier) -> bool {
        (*self & modifier).0 != 0
    }
}

impl std::ops::BitOr for Modifier {
    type Output = Modifier;
    fn bitor(self, rhs: Self) -> Self::Output {
        Modifier(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for Modifier {
    type Output = Modifier;
    fn bitand(self, rhs: Self) -> Self::Output {
        Modifier(self.0 & rhs.0)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Key {
    pub keycode: KeyCode,
    pub modifiers: Modifier,
}

impl Key {
    pub fn from_utf8(bytes: &[u8]) -> Key {
        let keycode = match bytes[0] {
            0x1b => KeyCode::Esc,
            0x7f => KeyCode::Backspace,
            0x9 => KeyCode::Tab,
            0x20 => KeyCode::Space,
            0xd | 0xa => KeyCode::Enter,
            _ => {
                let c = str::from_utf8(bytes).unwrap().chars().next().unwrap();
                KeyCode::Char(c)
            }
        };
        Key {
            keycode,
            modifiers: Modifier::NONE,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Key> {
        if bytes.is_empty() {
            return None;
        }

        match bytes[0] {
            b'\x1b' => {
                if bytes.len() == 1 {
                    Some(KeyCode::Esc.into())
                } else {
                    if bytes[1] == b'[' {
                        if bytes.len() == 2 {
                            None
                        } else {
                            match bytes[2] {
                                b'D' => Some(KeyCode::Left.into()),
                                b'C' => Some(KeyCode::Right.into()),
                                b'A' => Some(KeyCode::Up.into()),
                                b'B' => Some(KeyCode::Down.into()),
                                b'H' => Some(KeyCode::Home.into()),
                                b'F' => Some(KeyCode::End.into()),
                                _ => None,
                            }
                        }
                    } else {
                        None
                    }
                }
            }
            b'\r' => Some(KeyCode::Enter.into()),
            b'\t' => Some(KeyCode::Tab.into()),
            0x7f => Some(KeyCode::Backspace.into()),
            0x0 => Some(Key {
                keycode: KeyCode::Enter,
                modifiers: Modifier::CTRL,
            }),
            c @ 0x1..=0x1a => Some(Key {
                keycode: KeyCode::Char((c - 0x1 + b'a') as char),
                modifiers: Modifier::CTRL,
            }),
            c @ 0x1c..=0x1f => Some(Key {
                keycode: KeyCode::Char((c - 0x1c + b'4') as char),
                modifiers: Modifier::CTRL,
            }),
            _ => {
                let chars = str::from_utf8(bytes).ok()?;
                let first = chars.chars().next()?;
                if first.is_uppercase() {
                    Some(Key {
                        keycode: KeyCode::Char(first),
                        modifiers: Modifier::SHIFT,
                    })
                } else {
                    Some(KeyCode::Char(first).into())
                }
            }
        }
    }
}

impl From<KeyCode> for Key {
    fn from(value: KeyCode) -> Self {
        Key {
            keycode: value,
            modifiers: Modifier::NONE,
        }
    }
}

impl AsRef<Key> for Key {
    fn as_ref(&self) -> &Key {
        self
    }
}

#[derive(Debug)]
pub enum Event {
    Stdin(Vec<u8>),
    Resize((u16, u16)),

    Tick,

    /// push a new notification to the app instance to display
    /// to the user
    Notification((Notification, Duration)),

    /// allow setting the user from outside the application
    /// or from a view
    SetUser(User),
    Render,
}
