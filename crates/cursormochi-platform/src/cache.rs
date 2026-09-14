use cursormochi_core::Variant;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};
struct Entry {
    path: PathBuf,
    input: Vec<u8>,
    variants: Vec<Variant>,
    charge: usize,
}
/// Content-validated LRU: callers still open and bound-read each source before reuse.
/// Charges both input bytes and decoded pixel storage; textures are owned by GTK.
pub(crate) struct Cache {
    entries: VecDeque<Entry>,
    used: usize,
    budget: usize,
}
impl Cache {
    pub fn new(budget: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            used: 0,
            budget,
        }
    }
    pub fn get(&mut self, path: &Path, input: &[u8]) -> Option<Vec<Variant>> {
        let index = self
            .entries
            .iter()
            .position(|e| e.path == path && e.input == input)?;
        let entry = self.entries.remove(index)?;
        let result = entry.variants.clone();
        self.entries.push_back(entry);
        Some(result)
    }
    pub fn insert(&mut self, path: PathBuf, input: Vec<u8>, variants: Vec<Variant>) {
        let Some(charge) = variants
            .iter()
            .flat_map(|v| &v.frames)
            .try_fold(input.len(), |n, f| n.checked_add(f.rgba.len()))
        else {
            return;
        };
        if charge > self.budget {
            return;
        }
        // A replaced file does not keep its previous content alive in the cache.
        if let Some(i) = self.entries.iter().position(|e| e.path == path)
            && let Some(e) = self.entries.remove(i)
        {
            self.used -= e.charge;
        }
        while self.used > self.budget - charge {
            if let Some(e) = self.entries.pop_front() {
                self.used -= e.charge
            } else {
                break;
            }
        }
        self.used += charge;
        self.entries.push_back(Entry {
            path,
            input,
            variants,
            charge,
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn byte_budget_lru_and_content_validation() {
        let mut c = Cache::new(8);
        c.insert("a".into(), vec![1; 4], vec![]);
        c.insert("b".into(), vec![2; 4], vec![]);
        assert!(c.get(Path::new("a"), &[1; 4]).is_some());
        c.insert("c".into(), vec![3; 4], vec![]);
        assert!(c.get(Path::new("b"), &[2; 4]).is_none());
        assert!(c.get(Path::new("a"), &[9; 4]).is_none());
        assert_eq!(c.used, 8);
        c.insert("a".into(), vec![9; 2], vec![]);
        assert_eq!(c.used, 6);
        c.insert("large".into(), vec![0; 9], vec![]);
        assert_eq!(c.used, 6);
    }
}
