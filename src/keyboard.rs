use crate::typing::Next;

/// A keyboard layout on an ISO board: the unshifted character of each key, row by row
/// (number row, top, home, bottom). Fingers depend on the physical position only, so they are shared.
pub struct Layout {
    pub name: &'static str,
    pub rows: [&'static str; 4],
}

pub const LAYOUTS: &[Layout] = &[
    Layout {
        name: "qwerty",
        rows: ["`1234567890-=", "qwertyuiop[]", "asdfghjkl;'#", "\\zxcvbnm,./"],
    },
    Layout {
        name: "azerty",
        rows: ["²&é\"'(-è_çà)=", "azertyuiop^$", "qsdfghjklmù*", "<wxcvbn,;:!"],
    },
    Layout {
        name: "qwertz",
        rows: ["^1234567890ß´", "qwertzuiopü+", "asdfghjklöä#", "<yxcvbnm,.-"],
    },
    Layout {
        name: "colemak",
        rows: ["`1234567890-=", "qwfpgjluy;[]", "arstdhneio'#", "\\zxcvbkm,./"],
    },
    Layout {
        name: "dvorak",
        rows: ["`1234567890[]", "',.pyfgcrl/=", "aoeuidhtns-#", "\\;qjkxbmwvz"],
    },
];

/// Finger per key, same shape as `Layout::rows`. 0-3 left pinky..index, 4-7 right index..pinky.
const FINGERS: [&[u8]; 4] = [
    &[0, 0, 1, 2, 3, 3, 4, 4, 5, 6, 7, 7, 7],
    &[0, 1, 2, 3, 3, 4, 4, 5, 6, 7, 7, 7],
    &[0, 1, 2, 3, 3, 4, 4, 5, 6, 7, 7, 7],
    &[0, 0, 1, 2, 3, 3, 4, 4, 5, 6, 7],
];
pub const THUMB: u8 = 8;
const LEFT_PINKY: u8 = 0;
const RIGHT_PINKY: u8 = 7;

pub const FINGER_NAMES: [&str; 9] = [
    "left pinky",
    "left ring",
    "left middle",
    "left index",
    "right index",
    "right middle",
    "right ring",
    "right pinky",
    "thumb",
];

pub fn finger(row: usize, col: usize) -> u8 {
    FINGERS[row][col]
}

#[derive(Debug, PartialEq)]
pub struct Key {
    pub row: usize,
    pub col: usize,
    pub finger: u8,
    pub shift: bool,
}

impl Layout {
    pub fn by_name(name: &str) -> Option<usize> {
        LAYOUTS.iter().position(|l| l.name == name)
    }

    /// Where `c` is typed. Uppercase letters are their lowercase key plus shift.
    // shortcut: only unshifted keys and shifted letters; add shifted symbols (and AZERTY digits,
    // dead keys for ê/ü...) when punctuation and numbers modes land.
    pub fn find(&self, c: char) -> Option<Key> {
        let lower = c.to_lowercase().next()?;
        self.rows.iter().enumerate().find_map(|(row, keys)| {
            let col = keys.chars().position(|k| k == lower)?;
            Some(Key {
                row,
                col,
                finger: finger(row, col),
                shift: lower != c,
            })
        })
    }
}

/// What to press next and with which fingers.
#[derive(Debug, PartialEq)]
pub struct Hint {
    /// The key on the board, None for space, backspace or a character missing from the layout.
    pub key: Option<Key>,
    /// The finger for the key, then the shift pinky if needed.
    pub fingers: Vec<u8>,
}

impl Layout {
    pub fn hint(&self, next: Next) -> Hint {
        match next {
            Next::Backspace => Hint {
                key: None,
                fingers: vec![RIGHT_PINKY],
            },
            Next::Char(' ') => Hint {
                key: None,
                fingers: vec![THUMB],
            },
            Next::Char(c) => match self.find(c) {
                Some(k) => {
                    let mut fingers = vec![k.finger];
                    if k.shift {
                        // Shift with the other hand's pinky.
                        fingers.push(if k.finger < 4 { RIGHT_PINKY } else { LEFT_PINKY });
                    }
                    Hint {
                        key: Some(k),
                        fingers,
                    }
                }
                None => Hint {
                    key: None,
                    fingers: vec![],
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints() {
        let q = &LAYOUTS[0];
        assert_eq!(q.hint(Next::Char('f')).fingers, vec![3]);
        assert_eq!(q.hint(Next::Char('F')).fingers, vec![3, RIGHT_PINKY]);
        assert_eq!(q.hint(Next::Char('P')).fingers, vec![7, LEFT_PINKY]);
        assert_eq!(
            q.hint(Next::Char(' ')),
            Hint {
                key: None,
                fingers: vec![THUMB]
            }
        );
        assert_eq!(q.hint(Next::Backspace).fingers, vec![RIGHT_PINKY]);
        assert!(q.hint(Next::Char('ê')).fingers.is_empty());
    }

    #[test]
    fn rows_match_finger_map() {
        for l in LAYOUTS {
            for (row, keys) in l.rows.iter().enumerate() {
                assert_eq!(keys.chars().count(), FINGERS[row].len(), "{} row {row}", l.name);
            }
        }
    }

    #[test]
    fn home_row_fingers() {
        let qwerty = &LAYOUTS[0];
        assert_eq!(
            FINGER_NAMES[qwerty.find('f').unwrap().finger as usize],
            "left index"
        );
        assert_eq!(
            FINGER_NAMES[qwerty.find('j').unwrap().finger as usize],
            "right index"
        );
        assert_eq!(qwerty.find('a').unwrap().finger, 0);
        // Same key position, different letter on AZERTY.
        let azerty = &LAYOUTS[Layout::by_name("azerty").unwrap()];
        assert_eq!(
            azerty.find('q').unwrap(),
            Key {
                row: 2,
                col: 0,
                finger: 0,
                shift: false
            }
        );
        assert_eq!(azerty.find('é').unwrap().finger, 1);
    }

    #[test]
    fn uppercase_needs_shift() {
        let k = LAYOUTS[0].find('T').unwrap();
        assert!(k.shift);
        assert_eq!((k.row, k.col), (1, 4));
        assert_eq!(LAYOUTS[0].find('ê'), None);
    }
}
