use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// This is the constant 0b11111 << 59.
/// Used to extract 5 most significant bits from a u64.
const MOST_SIG: u64 = 17870283321406128128;

/// Implementation of a Hash Array Mapped Trie in Rust.
#[derive(Debug)]
pub struct HAMT<K, V> {
    root: Rc<HAMTNode<K, V>>,
}

// We can derive Clone automatically, as we are using Rc which supports clone.
#[derive(Clone, Debug)]
enum HAMTNodeEntry<K, V> {
    // Key, value
    Value(K, V),
    Node(Rc<HAMTNode<K, V>>),
    Chained(Vec<(K, V)>),
}

/// An internal node of a [`HAMT`](HAMT).
struct HAMTNode<K, V> {
    presence_map: u32,
    entries: Vec<HAMTNodeEntry<K, V>>,
}

/// Hash the given key using the rust `DefaultHasher`.
fn hash_key<K: Hash>(key: &K) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// Given a 'presence map', and an index between 0 and 31 (inclusive), 
/// compute what location the index will be in the entries vector.
fn get_entries_index(presence_map: u32, index: u32) -> usize {
    if index == 0 {
        0
    } else {
        (presence_map & ((1 << index) - 1)).count_ones() as usize
    }
}

/// Insert an entry into a vector chain. This will replace the existing value for that key, if one exists.
fn insert_chained<K: Eq + Clone, V: Clone>(vec: &Vec<(K, V)>, key: K, value: V) -> Vec<(K, V)> {
    let mut new_vec = vec.to_vec();
    for i in new_vec.iter_mut() {
        if i.0 == key {
            i.1 = value;
            return new_vec;
        }
    }
    new_vec.insert(0, (key, value));
    return new_vec;
}

/// Get the height of the subtree
fn get_height<K, V>(node: &HAMTNode<K, V>) -> u32 {
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

/// This is a key method: if called, there are conflicting hashed keys that need to be inserted
/// at the current level. If the conflict occurs at the 12th level or lower,
/// then the entry can point to a new node, which is constructed manually (we can predict what the new
/// lower node can look like because we know both keys that it should store).
/// If we are at the 13th level, then the data structure produces a chain instead.
/// 
/// Note that this can happen recursively, if the hashes of the keys share a prefix with more than 5 bits
/// starting at the current level.
fn create_split_entry<K, V>(
    key1: K,
    hashed_key1: u64,
    val1: V,
    key2: K,
    hashed_key2: u64,
    val2: V,
    level: u32,
) -> HAMTNodeEntry<K, V> {
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
            // Otherwise, create the node with only these two keys
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

/// Main method implementing insert at the current node.
/// Level keeps track of how deep in the tree we are.
fn insert_at_node<K: Hash + Eq + Clone, V: Clone>(
    node: &HAMTNode<K, V>,
    key: K,
    cur_hashed_key: u64,
    value: V,
    level: u32,
) -> HAMTNode<K, V> {
    let most_sig = ((cur_hashed_key & MOST_SIG) >> 59) as u32;
    let key_present = (node.presence_map >> most_sig) & 1;
    let entries_index = get_entries_index(node.presence_map, most_sig);
    // Check if there is a key present in the node whose 5 most significant bits conflict withe current key's.
    if key_present == 0 {
        // If the key is not present in the node, then the insert is more straightforward.
        // Copy the entries, insert the key and update the presence map.
        let mut new_entries = node.entries.to_vec();

        new_entries.insert(entries_index, HAMTNodeEntry::Value(key, value));
        return HAMTNode {
            presence_map: node.presence_map | (1 << most_sig),
            entries: new_entries,
        };
    } else {
        // If there is a conflicting key present, then we need to figure out how to update things
        // depending on the entry for that key prefix.
        let entry = &node.entries[entries_index];
        match entry {
            HAMTNodeEntry::Value(other_key, other_value) => {
                // If there is a value in the entry
                if other_key == &key {
                    // If it is for the same key, then just replace the value
                    let mut new_entries = node.entries.to_vec();
                    new_entries[entries_index] = HAMTNodeEntry::Value(key, value);
                    return HAMTNode {
                        presence_map: node.presence_map,
                        entries: new_entries,
                    };
                } else {
                    // Otherwise, we need to split this entry.
                    let mut new_entries = node.entries.to_vec();
                    let other_hashed_key = hash_key(other_key) << (5 * (level + 1));
                    new_entries[entries_index] = create_split_entry(
                        key,
                        cur_hashed_key << 5,
                        value,
                        other_key.clone(),
                        other_hashed_key,
                        other_value.clone(),
                        level + 1,
                    );
                    return HAMTNode {
                        presence_map: node.presence_map,
                        entries: new_entries,
                    };
                }
            }
            HAMTNodeEntry::Chained(vec) => {
                // In a chain, we insert the key into the chain (replacing the existing value for that key if needed)
                let new_chain = insert_chained(vec, key, value);
                let mut new_entries = node.entries.to_vec();
                new_entries[entries_index] = HAMTNodeEntry::Chained(new_chain);
                return HAMTNode {
                    presence_map: node.presence_map,
                    entries: new_entries,
                };
            }
            HAMTNodeEntry::Node(child_node) => {
                // If the entry points to another node, then we need to insert within that node.
                let new_key = cur_hashed_key << 5;
                let new_node = insert_at_node(child_node, key, new_key, value, level + 1);
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

/// Remove the given key at the node.
fn remove_at_node<K: Eq + Clone, V: Clone>(
    node: Rc<HAMTNode<K, V>>,
    key: K,
    cur_hashed_key: u64
) -> Rc<HAMTNode<K, V>> {
    let most_sig = ((cur_hashed_key & MOST_SIG) >> 59) as u32;
    let key_present = (node.presence_map >> most_sig) & 1;
    let entries_index = get_entries_index(node.presence_map, most_sig);
    if key_present == 0 {
        // If the key is not present at this level, we need to do nothing, so return the node
        node
    } else {
        let entry = &node.entries[entries_index];
        // Like the insert, what we need to do if the key's prefix is present depends on the entry for that
        // prefix
        let ret_node = match entry {
            HAMTNodeEntry::Chained(vec) => {
                // If it is a chain, then go through the chain and remove the key if it exists.
                let mut new_chain = vec.to_vec();
                let mut new_entries = node.entries.to_vec();
                let loc = new_chain.iter().position(|(k, _)| *k == key);
                match loc {
                    Some(i) => {
                        new_chain.remove(i);                       
                        if new_chain.len() == 0 {
                            // One special case: if the chain is now empty after removing the key,
                            // then the containing node can be updated to remove the entry pointing to
                            // that chain.
                            new_entries.remove(entries_index);
                            let node = HAMTNode {
                                presence_map: node.presence_map ^ (1 << most_sig),
                                entries: new_entries
                            };
                            Rc::new(node)
                        } else {
                            new_entries[entries_index] = HAMTNodeEntry::Chained(new_chain);
                            let node = HAMTNode {
                                presence_map: node.presence_map,
                                entries: new_entries
                            };
                            Rc::new(node)
                        }
                    }
                    None => {
                        node
                    }
                }
            }
            HAMTNodeEntry::Node(next_node) => {
                // If it is a node, then recurse through removing the node
                let new_node = remove_at_node(
                    Rc::clone(next_node), key, cur_hashed_key << 5
                );
                let mut new_entries = node.entries.to_vec();
                if new_node.presence_map == 0 {
                    // Also clean up the node from its parent's presence map if the node is entry.
                    new_entries.remove(entries_index);
                    let node = HAMTNode {
                        presence_map: node.presence_map ^ (1 << most_sig),
                        entries: new_entries
                    };
                    Rc::new(node)
                } else {
                    new_entries[entries_index] = HAMTNodeEntry::Node(new_node);
                    let node = HAMTNode {
                        presence_map: node.presence_map,
                        entries: new_entries
                    };
                    Rc::new(node)
                }
            }
            HAMTNodeEntry::Value(k, _) => {
                // If the entry is a value, this is the most direct case.
                if *k == key {
                    // If the key matches, then remove the entry.
                    let mut new_entries = node.entries.to_vec();
                    new_entries.remove(entries_index);
                    let node = HAMTNode {
                        presence_map: node.presence_map ^ (1 << most_sig),
                        entries: new_entries
                    };
                    Rc::new(node)
                } else {
                    node
                }
            }
        };
        ret_node
    }
}

impl<K, V> HAMT<K, V> {
    /// Construct a new HAMT.
    pub fn new() -> Self {
        let root_node = HAMTNode {
            presence_map: 0,
            entries: Vec::new(),
        };
        Self {
            root: Rc::new(root_node),
        }
    }

    // Get the height of the HAMT.
    pub fn height(&self) -> u32 {
        get_height(&self.root)
    }
}

impl<K, V> HAMT<K, V>
where
    K: Eq + Hash,
{
    /// Get the value stored at key if it exists, otherwise return `None`.
    pub fn get(&self, key: K) -> Option<&V> {
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
                HAMTNodeEntry::Value(k, v) => {
                    if *k == key {
                        break Some(&v);
                    } else {
                        break None;
                    }
                }
                HAMTNodeEntry::Chained(vec) => {
                    for (k, v) in vec {
                        if *k == key {
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

    /// Check if the HAMT contains the given key, and return `true` if so and `false` if not.
    pub fn contains_key(&self, key: K) -> bool {
        let hashed_key = hash_key(&key);
        let mut cur_node = &self.root;
        let mut cur_key = hashed_key;
        // The main body of this is very similar to `get`, only we just finish when we find
        // a matching key.
        'main: loop {
            let most_sig = ((cur_key & MOST_SIG) >> 59) as u32;

            let key_present = (cur_node.presence_map >> most_sig) & 1;
            if key_present == 0 {
                break false;
            }
            let entries_index = get_entries_index(cur_node.presence_map, most_sig);
            let entry = &cur_node.entries[entries_index];
            match entry {
                HAMTNodeEntry::Value(k, _) => {
                    break *k == key;
                }
                HAMTNodeEntry::Chained(vec) => {
                    for (k, _) in vec {
                        if *k == key {
                            break 'main true;
                        }
                    }
                }
                HAMTNodeEntry::Node(next_node) => {
                    cur_node = &next_node;
                    cur_key = cur_key << 5;
                }
            }
        }
    }
}

impl<K, V> HAMT<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone
{
    /// Create a HAMT from the given array of pairs.
    pub fn from<const N: usize>(items: [(K, V); N]) -> Self {
        let mut map = Self::new();
        for (k, v) in items {
            map = map.insert(k, v)
        }
        map
    }

    /// Insert the given key and value in to the map.
    /// Return a new HAMT, with the existing one unaffected.
    pub fn insert(&self, key: K, value: V) -> HAMT<K, V> {
        let hashed_key = hash_key(&key);
        let new_root = insert_at_node(&self.root, key, hashed_key, value, 0);
        HAMT {
            root: Rc::new(new_root),
        }
    }

    /// Remove the given key from the map, if it is present.
    /// Return a HAMT, with the existing one unaffected.
    pub fn remove(&self, key: K) -> HAMT<K, V> {
        let hashed_key = hash_key(&key);
        let new_root = remove_at_node(Rc::clone(&self.root), key, hashed_key);
        HAMT { root: new_root }
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for HAMTNode<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HAMTNode")
            .field("presence_map", &format!("{:#b}", &self.presence_map))
            .field("entries", &self.entries)
            .finish()
    }
}

impl<K, V> Clone for HAMT<K, V>
where
    K: Clone,
    V: Clone
{
    fn clone(&self) -> Self {
        Self {
            root: Rc::clone(&self.root)
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::HAMT;

    fn setup_big_map() -> (i32, HAMT<i32, i32>) {
        let num_keys = 10000;
        let mut map = HAMT::new();
        for k in 1..num_keys {
            map = map.insert(k, -k);
        }
        (num_keys, map)
    }

    #[test]
    fn set_then_get() {
        let (n, map) = setup_big_map();
        
        for k in 1..n {
            let val = map.get(k).unwrap();
            assert_eq!(*val, -k);
        }
    }

    #[test]
    fn set_immutability() {
        let (n, map) = setup_big_map();

        let mut map2 = map.clone();
        for k in n..(2*n) {
            map2 = map2.insert(k, -k);
        }
        for k in n..(2*n) {
            assert!(!map.contains_key(k));
            assert!(map2.contains_key(k));
        }
    }

    #[test]
    fn from() {
        let map = HAMT::from([
            ("a", 1),
            ("b", 2)
        ]);
        assert_eq!(*map.get("a").unwrap(), 1);
        assert_eq!(*map.get("b").unwrap(), 2);
        assert_eq!(map.get("c"), None);
    }

    #[test]
    fn contains_key() {
        let map = HAMT::from([
            ("a", 1),
            ("b", 2)
        ]);
        assert!(map.contains_key("a"));
        assert!(map.contains_key("b"));
        assert!(!map.contains_key("c"));
    }

    #[test]
    fn big_contains_key() {
        let (n, map) = setup_big_map();
        for k in 1..n {
            assert!(map.contains_key(k));
        }
        assert!(!map.contains_key(0));
        assert!(!map.contains_key(-1));
        assert!(!map.contains_key(n+1));
    }

    #[test]
    fn big_remove() {
        let (n, mut map) = setup_big_map();
        for k in (1..n).step_by(2) {
            map = map.remove(k);
        }
        for k in (1..n).step_by(2) {
            assert!(!map.contains_key(k));
        }
        for k in (2..n).step_by(2) {
            assert!(map.contains_key(k));
        }
    }

    #[test]
    fn remove_immutability() {
        let (n, map) = setup_big_map();

        let mut map2 = map.clone();
        for k in n..(2*n) {
            map2 = map2.insert(k, -k);
        }
        for k in (1..n).step_by(2) {
            map2 = map2.remove(k);
        }

        for k in (1..n).step_by(2) {
            assert!(map.contains_key(k));
            assert!(!map2.contains_key(k));
        }
    }

}
