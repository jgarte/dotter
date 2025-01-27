use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use config;

#[derive(Debug)]
pub struct FileState {
    pub desired_symlinks: BTreeSet<SymlinkDescription>,
    pub desired_templates: BTreeSet<TemplateDescription>,
    pub existing_symlinks: BTreeSet<SymlinkDescription>,
    pub existing_templates: BTreeSet<TemplateDescription>,
}

#[derive(Debug, Clone)]
pub struct SymlinkDescription {
    pub source: PathBuf,
    pub target: config::SymbolicTarget,
}

#[derive(Debug, Clone)]
pub struct TemplateDescription {
    pub source: PathBuf,
    pub target: config::TemplateTarget,
    pub cache: PathBuf,
}

// For use in FileState's Sets
impl std::cmp::PartialEq for SymlinkDescription {
    fn eq(&self, other: &SymlinkDescription) -> bool {
        self.source == other.source && self.target.target == other.target.target
    }
}
impl std::cmp::Eq for SymlinkDescription {}
impl std::cmp::PartialOrd for SymlinkDescription {
    fn partial_cmp(&self, other: &SymlinkDescription) -> Option<std::cmp::Ordering> {
        Some(
            self.source
                .cmp(&other.source)
                .then(self.target.target.cmp(&other.target.target)),
        )
    }
}
impl std::cmp::Ord for SymlinkDescription {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl std::cmp::PartialEq for TemplateDescription {
    fn eq(&self, other: &TemplateDescription) -> bool {
        self.source == other.source && self.target.target == other.target.target
    }
}
impl std::cmp::Eq for TemplateDescription {}
impl std::cmp::PartialOrd for TemplateDescription {
    fn partial_cmp(&self, other: &TemplateDescription) -> Option<std::cmp::Ordering> {
        Some(
            self.source
                .cmp(&other.source)
                .then(self.target.target.cmp(&other.target.target)),
        )
    }
}
impl std::cmp::Ord for TemplateDescription {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl TemplateDescription {
    pub fn apply_actions(&self, mut file: String) -> String {
        if let Some(ref append) = self.target.append {
            file = file + append;
        }
        if let Some(ref prepend) = self.target.prepend {
            file = prepend.to_string() + &file;
        }

        file
    }
}

impl std::fmt::Display for SymlinkDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "symlink {:?} -> {:?}", self.source, self.target.target)
    }
}

impl std::fmt::Display for TemplateDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "template {:?} -> {:?}", self.source, self.target.target)
    }
}

impl FileState {
    pub fn new(
        desired_symlinks: BTreeMap<PathBuf, config::SymbolicTarget>,
        desired_templates: BTreeMap<PathBuf, config::TemplateTarget>,
        existing_symlinks: BTreeMap<PathBuf, PathBuf>,
        existing_templates: BTreeMap<PathBuf, PathBuf>,
        cache_dir: PathBuf,
    ) -> FileState {
        FileState {
            desired_symlinks: Self::symlinks_to_set(desired_symlinks),
            desired_templates: Self::templates_to_set(desired_templates, &cache_dir),
            existing_symlinks: Self::symlinks_to_set(
                existing_symlinks
                    .into_iter()
                    .map(|(source, target)| {
                        (
                            source,
                            config::SymbolicTarget {
                                target,
                                owner: None,
                            },
                        )
                    })
                    .collect(),
            ),
            existing_templates: Self::templates_to_set(
                existing_templates
                    .into_iter()
                    .map(|(source, target)| {
                        (
                            source,
                            config::TemplateTarget {
                                target,
                                owner: None,
                                append: None,
                                prepend: None,
                            },
                        )
                    })
                    .collect(),
                &cache_dir,
            ),
        }
    }

    pub fn symlinks_to_set(
        symlinks: BTreeMap<PathBuf, config::SymbolicTarget>,
    ) -> BTreeSet<SymlinkDescription> {
        symlinks
            .into_iter()
            .map(|(source, target)| SymlinkDescription { source, target })
            .collect()
    }

    fn templates_to_set(
        templates: BTreeMap<PathBuf, config::TemplateTarget>,
        cache_dir: &Path,
    ) -> BTreeSet<TemplateDescription> {
        templates
            .into_iter()
            .map(|(source, target)| TemplateDescription {
                source: source.clone(),
                target,
                cache: cache_dir.join(&source),
            })
            .collect()
    }

    pub fn deleted_files(&self) -> (Vec<SymlinkDescription>, Vec<TemplateDescription>) {
        (
            self.existing_symlinks
                .difference(&self.desired_symlinks)
                .cloned()
                .collect(),
            self.existing_templates
                .difference(&self.desired_templates)
                .cloned()
                .collect(),
        )
    }
    pub fn new_files(&self) -> (Vec<SymlinkDescription>, Vec<TemplateDescription>) {
        (
            self.desired_symlinks
                .difference(&self.existing_symlinks)
                .cloned()
                .collect(),
            self.desired_templates
                .difference(&self.existing_templates)
                .cloned()
                .collect(),
        )
    }
    pub fn old_files(&self) -> (Vec<SymlinkDescription>, Vec<TemplateDescription>) {
        (
            self.desired_symlinks
                .intersection(&self.existing_symlinks)
                .cloned()
                .collect(),
            self.desired_templates
                .intersection(&self.existing_templates)
                .cloned()
                .collect(),
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_file_state_symlinks_only() {
        let mut existing_symlinks = BTreeMap::new();
        existing_symlinks.insert("file1s".into(), "file1t".into()); // Same
        existing_symlinks.insert("file2s".into(), "file2t".into()); // Deleted
        existing_symlinks.insert("file3s".into(), "file3t".into()); // Target change

        let mut desired_symlinks = BTreeMap::new();
        desired_symlinks.insert("file1s".into(), "file1t".into()); // Same
        desired_symlinks.insert("file3s".into(), "file0t".into()); // Target change
        desired_symlinks.insert("file5s".into(), "file5t".into()); // New

        let state = FileState::new(
            desired_symlinks,
            Default::default(),
            existing_symlinks,
            Default::default(),
            "cache".into(),
        );

        assert_eq!(
            state.deleted_files(),
            (
                vec![
                    SymlinkDescription {
                        source: "file2s".into(),
                        target: "file2t".into(),
                    },
                    SymlinkDescription {
                        source: "file3s".into(),
                        target: "file3t".into(),
                    }
                ],
                Vec::new()
            ),
            "deleted files correct"
        );
        assert_eq!(
            state.new_files(),
            (
                vec![
                    SymlinkDescription {
                        source: "file3s".into(),
                        target: "file0t".into(),
                    },
                    SymlinkDescription {
                        source: "file5s".into(),
                        target: "file5t".into(),
                    },
                ],
                Vec::new()
            ),
            "new files correct"
        );
        assert_eq!(
            state.old_files(),
            (
                vec![SymlinkDescription {
                    source: "file1s".into(),
                    target: "file1t".into(),
                }],
                Vec::new()
            ),
            "old files correct"
        );
    }

    #[test]
    fn test_file_state_complex() {
        let mut existing_templates = BTreeMap::new();
        existing_templates.insert("file1s".into(), "file1t".into()); // Same
        existing_templates.insert("file2s".into(), "file2t".into()); // Deleted
        existing_templates.insert("file3s".into(), "file3t".into()); // Target change

        let mut desired_templates = BTreeMap::new();
        desired_templates.insert("file1s".into(), "file1t".into()); // Same
        desired_templates.insert("file3s".into(), "file0t".into()); // Target change
        desired_templates.insert("file5s".into(), "file5t".into()); // New

        let state = FileState::new(
            Default::default(),
            desired_templates,
            Default::default(),
            existing_templates,
            "cache".into(),
        );
        todo!()
    }
}
