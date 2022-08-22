use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// Implementation of a Hash Array Mapped Trie in Rust.
#[derive(Debug)]
pub struct HAMT {
    root: Rc<HAMTNode>,
}

/// This is the constant 0b11111 << 59.
/// Used to extract 5 most significant bits from a u64.
const MOST_SIG: u64 = 17870283321406128128;

fn hash_key(key: &u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

fn get_entries_index(presence_map: u32, index: u32) -> usize {
    if index == 0 {
        0
    } else {
        (presence_map & ((1 << index) - 1)).count_ones() as usize
    }
}

fn set_chained(vec: &Vec<(u64, i32)>, key: u64, value: i32) -> Vec<(u64, i32)> {
    let mut new_vec = vec.to_vec();
    for i in new_vec.iter_mut() {
        let (k, v) = *i;
        if k == key {
            *i = (k, v);
            return new_vec;
        }
    }
    new_vec.insert(0, (key, value));
    return new_vec;
}

fn get_height(node: &HAMTNode) -> u32 {
    if node.presence_map == 0 {
        0
    } else {
        let mut max_child_depth = 0;
        for entry in node.entries.iter() {
            let entry_depth = match entry {
                HAMTNodeEntry::Value(_, _) => 0,
                HAMTNodeEntry::Chained(_) => 1,
                HAMTNodeEntry::Node(child_node) => get_height(child_node),
            };
            if entry_depth > max_child_depth {
                max_child_depth = entry_depth;
            }
        }
        max_child_depth + 1
    }
}

///
///
///
fn create_split_entry(
    key1: u64,
    hashed_key1: u64,
    val1: i32,
    key2: u64,
    hashed_key2: u64,
    val2: i32,
    level: u32,
) -> HAMTNodeEntry {
    // If at the 13th level, there are no more bits in the keys to read.
    // Then a new chain is created
    if level == 13 {
        let chained_vec = vec![(key1, val1), (key2, val2)];
        return HAMTNodeEntry::Chained(chained_vec);
    } else {
        let key1_frag = ((hashed_key1 & MOST_SIG) >> 59) as u32;
        let key2_frag = ((hashed_key2 & MOST_SIG) >> 59) as u32;
        let node = if key1_frag == key2_frag {
            // If the next fragments are still the same, then need to split even further
            let next_split_entry = create_split_entry(
                key1,
                hashed_key1 << 5,
                val1,
                key2,
                hashed_key2 << 5,
                val2,
                level + 1,
            );
            HAMTNode {
                presence_map: 1 << key1_frag,
                entries: vec![next_split_entry],
            }
        } else {
            let entries = if key1_frag < key2_frag {
                vec![
                    HAMTNodeEntry::Value(key1, val1),
                    HAMTNodeEntry::Value(key2, val2),
                ]
            } else {
                vec![
                    HAMTNodeEntry::Value(key2, val2),
                    HAMTNodeEntry::Value(key1, val1),
                ]
            };
            HAMTNode {
                presence_map: (1 << key1_frag) | (1 << key2_frag),
                entries: entries,
            }
        };
        return HAMTNodeEntry::Node(Rc::new(node));
    }
}

fn set_at_node(node: &HAMTNode, key: u64, cur_hashed_key: u64, value: i32, level: u32) -> HAMTNode {
    let most_sig = ((cur_hashed_key & MOST_SIG) >> 59) as u32;
    let key_present = (node.presence_map >> most_sig) & 1;
    let entries_index = get_entries_index(node.presence_map, most_sig);
    if key_present == 0 {
        let mut new_entries = node.entries.to_vec();

        new_entries.insert(entries_index, HAMTNodeEntry::Value(key, value));
        return HAMTNode {
            presence_map: node.presence_map | (1 << most_sig),
            entries: new_entries,
        };
    } else {
        let entry = &node.entries[entries_index];
        match entry {
            HAMTNodeEntry::Value(other_key, other_value) => {
                if other_key == &key {
                    let mut new_entries = node.entries.to_vec();
                    new_entries[entries_index] = HAMTNodeEntry::Value(key, value);
                    return HAMTNode {
                        presence_map: node.presence_map,
                        entries: new_entries,
                    };
                } else {
                    let mut new_entries = node.entries.to_vec();
                    let other_hashed_key = hash_key(other_key) << (5 * (level + 1));
                    new_entries[entries_index] = create_split_entry(
                        key,
                        cur_hashed_key << 5,
                        value,
                        *other_key,
                        other_hashed_key,
                        *other_value,
                        level + 1,
                    );
                    return HAMTNode {
                        presence_map: node.presence_map,
                        entries: new_entries,
                    };
                }
            }
            HAMTNodeEntry::Chained(vec) => {
                let new_chain = set_chained(vec, key, value);
                let mut new_entries = node.entries.to_vec();
                new_entries[entries_index] = HAMTNodeEntry::Chained(new_chain);
                return HAMTNode {
                    presence_map: node.presence_map,
                    entries: new_entries,
                };
            }
            HAMTNodeEntry::Node(child_node) => {
                let new_key = cur_hashed_key << 5;
                let new_node = set_at_node(
                    child_node, key, new_key, value, level + 1
                );
                let mut new_entries = node.entries.to_vec();
                new_entries[entries_index] = HAMTNodeEntry::Node(Rc::new(new_node));
                return HAMTNode {
                    presence_map: node.presence_map,
                    entries: new_entries,
                };
            }
        }
    }
}

fn delete_at_node(
    node: Rc<HAMTNode>,
    key: u64,
    cur_hashed_key: u64,
    value: i32,
    level: u32,
) -> Rc<HAMTNode> {
    let most_sig = ((cur_hashed_key & MOST_SIG) >> 59) as u32;
    let key_present = (node.presence_map >> most_sig) & 1;
    let entries_index = get_entries_index(node.presence_map, most_sig);
    if key_present == 0 {
        // If the key is not present at this level, return the node
        node
    } else {
        let new_node = HAMTNode {
            presence_map: 0,
            entries: Vec::new(),
        };
        Rc::new(new_node)
    }
}

impl HAMT {
    /// Construct a new HAMT.
    pub fn new() -> Self {
        let root_node = HAMTNode {
            presence_map: 0,
            entries: Vec::new(),
        };
        HAMT {
            root: Rc::new(root_node),
        }
    }

    pub fn get(&self, key: u64) -> Option<&i32> {
        // Hash the key first.
        let hashed_key = hash_key(&key);

        let mut cur_node = &self.root;
        let mut cur_key = hashed_key;
        'main: loop {
            // Get the 5 most significant bits of the key.
            // This will always be a number between 0 and 31.
            // We use this to index into the up to 32 entries of the node.
            // Casting to u32 is always safe, as after we bitshift we only have the 5 least
            // significant bits.
            let most_sig = ((cur_key & MOST_SIG) >> 59) as u32;

            // Is the key present?
            let key_present = (cur_node.presence_map >> most_sig) & 1;
            if key_present == 0 {
                break None;
            }
            // Count the number of present entries before this.
            // This will be the index in the entries array.
            // We assume we don't lose anything casting to usize, 
            // i.e. that usize is at least 5 bits.
            let entries_index = get_entries_index(cur_node.presence_map, most_sig);
            // We can unwrap, as we are guaranteed that the length of the vector
            // is at least the number of ones in the presence map.
            let entry = &cur_node.entries[entries_index];
            match entry {
                HAMTNodeEntry::Value(_, v) => {
                    break Some(&v);
                }
                HAMTNodeEntry::Chained(vec) => {
                    for (k, v) in vec.iter() {
                        if k == &key {
                            break 'main Some(&v);
                        }
                    }
                }
                HAMTNodeEntry::Node(new_node) => {
                    cur_node = &new_node;
                    // Move the key so the next 5 bits are in position
                    cur_key = cur_key << 5;
                }
            }
        }
    }

    pub fn set(&self, key: u64, value: i32) -> HAMT {
        let hashed_key = hash_key(&key);
        let new_root = set_at_node(
            &self.root, key, hashed_key, value, 0
        );
        HAMT {
            root: Rc::new(new_root),
        }
    }

    pub fn delete(&self, key: u64, value: i32) -> HAMT {
        let hashed_key = hash_key(&key);
        let new_root = delete_at_node(
            Rc::clone(&self.root), key, hashed_key, value, 0
        );
        HAMT { root: new_root }
    }

    pub fn height(&self) -> u32 {
        get_height(&self.root)
    }
}

// We can derive Clone automatically, as we are using Rc which supports clone.
#[derive(Clone, Debug)]
enum HAMTNodeEntry {
    // Key, value
    Value(u64, i32),
    Node(Rc<HAMTNode>),
    Chained(Vec<(u64, i32)>),
}

/// An internal node of a [`HAMT`](HAMT).
struct HAMTNode {
    presence_map: u32,
    entries: Vec<HAMTNodeEntry>,
}

impl fmt::Debug for HAMTNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HAMTNode")
            .field("presence_map", &format!("{:#b}", &self.presence_map))
            .field("entries", &self.entries)
            .finish()
    }
}
