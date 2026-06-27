use std::collections::{HashMap, hash_map::Entry};

use anyhow::{Result, bail};

use super::{ArchiveEntry, PakPath};

pub struct ArchivePathIndex {
    exact_entries: HashMap<String, usize>,
    normalized_entries: HashMap<String, Option<usize>>,
    directories: HashMap<String, Option<String>>,
}

impl ArchivePathIndex {
    pub fn new(entries: &[ArchiveEntry]) -> Result<Self> {
        let mut index = Self {
            exact_entries: HashMap::with_capacity(entries.len()),
            normalized_entries: HashMap::with_capacity(entries.len()),
            directories: HashMap::new(),
        };
        for (entry_index, entry) in entries.iter().enumerate() {
            index.insert_file(entry.name(), entry_index)?;
        }
        Ok(index)
    }

    pub fn resolve_entry_index(&self, pak_path: &PakPath) -> Option<usize> {
        if let Some(entry_index) = self.exact_entries.get(pak_path.as_str()).copied() {
            return Some(entry_index);
        }
        self.normalized_entries
            .get(&Self::normalize(pak_path.as_str()))
            .and_then(|entry_index| *entry_index)
    }

    pub fn canonicalize_for_insert(&self, pak_path: &PakPath) -> Result<PakPath> {
        let components = pak_path.as_str().split('/').collect::<Vec<_>>();
        if components.len() < 2 {
            return PakPath::new(pak_path.as_str().to_owned());
        }

        let mut incoming_directory = String::new();
        let mut canonical_components = Vec::<String>::with_capacity(components.len());
        for component in &components[..components.len() - 1] {
            if !incoming_directory.is_empty() {
                incoming_directory.push('/');
            }
            incoming_directory.push_str(component);

            if let Some(Some(canonical_directory)) =
                self.directories.get(&Self::normalize(&incoming_directory))
            {
                canonical_components.clear();
                canonical_components.extend(canonical_directory.split('/').map(str::to_owned));
            } else {
                canonical_components.push((*component).to_owned());
            }
        }
        canonical_components.push(components[components.len() - 1].to_owned());
        PakPath::new(canonical_components.join("/"))
    }

    pub fn insert_file(&mut self, name: &str, entry_index: usize) -> Result<()> {
        if self
            .exact_entries
            .insert(name.to_owned(), entry_index)
            .is_some()
        {
            bail!("duplicate pak entry name: {name}");
        }
        self.remember_normalized_entry(name, entry_index);
        self.remember_directories(name);
        Ok(())
    }

    fn remember_normalized_entry(&mut self, name: &str, entry_index: usize) {
        let key = Self::normalize(name);
        match self.normalized_entries.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(Some(entry_index));
            }
            Entry::Occupied(mut entry) => {
                if entry.get().is_some_and(|existing| existing != entry_index) {
                    entry.insert(None);
                }
            }
        }
    }

    fn remember_directories(&mut self, name: &str) {
        let mut directory = String::new();
        let mut components = name.split('/').peekable();
        while let Some(component) = components.next() {
            if components.peek().is_none() {
                break;
            }
            if !directory.is_empty() {
                directory.push('/');
            }
            directory.push_str(component);
            let key = Self::normalize(&directory);
            match self.directories.entry(key) {
                Entry::Vacant(entry) => {
                    entry.insert(Some(directory.clone()));
                }
                Entry::Occupied(mut entry) => {
                    if entry
                        .get()
                        .as_ref()
                        .is_some_and(|existing| existing != &directory)
                    {
                        entry.insert(None);
                    }
                }
            }
        }
    }

    fn normalize(value: &str) -> String {
        value.to_ascii_lowercase()
    }
}
