use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// Implementation of a Hash Array Mapped Trie in Rst.
pub struct HAMT {
  root: HAMTNode
}

// This is the constant 0b11111 << 61.
// Used to extract 5 most significant bits.
const MOST_SIG: u64 = 17870283321406128128;

fn hash_key(key: u64) -> u64 {
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

fn set_node(node: &HAMTNode, cur_key: u64, value: i32) -> HAMTNode {
  let most_sig = ((cur_key & MOST_SIG) >> 59) as u32;
  let key_present = (node.presence_map >> most_sig) & 1;
  let entries_index = get_entries_index(node.presence_map, most_sig);
  if key_present == 0 {
    let mut new_entries = node.entries.to_vec();

    new_entries.insert(
      entries_index, 
      HAMTNodeEntry::Value(value)
    );
    return HAMTNode {
      presence_map: node.presence_map | (1 << most_sig),
      entries: new_entries
    };
  } else {
    let entry = &node.entries[entries_index];
    match entry {
      HAMTNodeEntry::Value(_) => {
        let mut new_entries = node.entries.to_vec();
        new_entries[entries_index] = HAMTNodeEntry::Value(value);
        return HAMTNode {
          presence_map: node.presence_map,
          entries: new_entries
        };
      }
      HAMTNodeEntry::Node(child_node) => {
        let new_key = cur_key << 5;
        let new_node = set_node(child_node, new_key, value);
        let mut new_entries = node.entries.to_vec();
        new_entries[entries_index] = HAMTNodeEntry::Node(Rc::new(new_node));
        return HAMTNode {
          presence_map: node.presence_map,
          entries: new_entries
        };
      }
    }
  }
}

impl HAMT {
  
  /// Construct a new HAMT.
  pub fn new() -> Self {
    return HAMT { 
      root: HAMTNode {
        presence_map: 0,
        entries: Vec::new()
      }
     };
  }

  pub fn get(&self, key: u64) -> Option<&i32> {
    // Hash the key first.
    let hashed_key = hash_key(key);

    let mut cur_node = &self.root;
    let mut cur_key = hashed_key;
    loop {
      // Get the 5 most significant bits of the key.
      // This will always be a number between 0 and 31.
      // We use this to index into the up to 32 entries of the node.
      // Casting to u32 is always safe, as after we bitshift we only have the 5 least significant bits.
      let most_sig = ((cur_key & MOST_SIG) >> 59) as u32;

      // Is the key present?
      let key_present = (cur_node.presence_map >> most_sig) & 1;
      if key_present == 0 {
        break None;
      }
      // Count the number of present entries before this. This will be the index in the entries array.
      // We assume we don't lose anything casting to usize, i.e. that usize is at least 5 bits.
      let entries_index = get_entries_index(cur_node.presence_map, most_sig);
      // We can unwrap, as we are guaranteed that the length of the vector is at least the number of ones in the presence map.
      let entry = &cur_node.entries[entries_index];
      match entry {
        HAMTNodeEntry::Value(v) => {
          break Some(&v);
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
    let hashed_key = hash_key(key);
    println!("key hash {} {}", key, hashed_key);
    HAMT {
      root: set_node(&self.root, hashed_key, value)
    }
  }

}

// TODO for more speed, implement as union?
// We can derive Clone automatically, as we are using Rc which supports clone.
#[derive(Clone)]
enum HAMTNodeEntry {
  Value(i32),
  Node(Rc<HAMTNode>)
}

struct HAMTNode {
  /// Test
  presence_map: u32,
  entries: Vec<HAMTNodeEntry>
}
