use std::path::{Component, Path, PathBuf};

pub struct NaplPaths {
    pub ir_dir: PathBuf,
    pub src_dir: PathBuf,
    pub map_path: PathBuf,
    pub lock_path: PathBuf,
    pub gen_lock_path: PathBuf,
    pub journal_path: PathBuf,
    pub prompts_at_gen_dir: PathBuf,
    pub examples_dir: PathBuf,
    pub attribution_dir: PathBuf,
    pub ml_dir: PathBuf,
}

pub fn resolve_paths(root: &Path) -> NaplPaths {
    let napl_dir = root.join(".napl");
    NaplPaths {
        ir_dir: napl_dir.join("ir"),
        src_dir: napl_dir.join("src"),
        map_path: napl_dir.join("map.json"),
        lock_path: napl_dir.join("lock.json"),
        gen_lock_path: napl_dir.join("gen.lock"),
        journal_path: napl_dir.join("journal.jsonl"),
        prompts_at_gen_dir: napl_dir.join("prompts-at-gen"),
        examples_dir: root.join("examples"),
        attribution_dir: napl_dir.join("attribution"),
        ml_dir: napl_dir.join("mapl"),
    }
}

pub fn rel_to(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            Component::CurDir => None,
            Component::ParentDir => Some("..".to_string()),
            Component::RootDir => Some(String::new()),
            Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().into_owned()),
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_paths_places_fields_under_napl_dir() {
        let paths = resolve_paths(Path::new("/p"));
        assert_eq!(paths.ir_dir, PathBuf::from("/p/.napl/ir"));
        assert_eq!(paths.src_dir, PathBuf::from("/p/.napl/src"));
        assert_eq!(paths.map_path, PathBuf::from("/p/.napl/map.json"));
        assert_eq!(paths.lock_path, PathBuf::from("/p/.napl/lock.json"));
        assert_eq!(paths.gen_lock_path, PathBuf::from("/p/.napl/gen.lock"));
        assert_eq!(paths.journal_path, PathBuf::from("/p/.napl/journal.jsonl"));
        assert_eq!(
            paths.prompts_at_gen_dir,
            PathBuf::from("/p/.napl/prompts-at-gen")
        );
        assert_eq!(paths.attribution_dir, PathBuf::from("/p/.napl/attribution"));
    }

    #[test]
    fn examples_dir_sits_beside_napl_not_inside_it() {
        let paths = resolve_paths(Path::new("/p"));
        assert_eq!(paths.examples_dir, PathBuf::from("/p/examples"));
    }

    #[test]
    fn ml_dir_is_named_mapl() {
        let paths = resolve_paths(Path::new("/p"));
        assert_eq!(paths.ml_dir, PathBuf::from("/p/.napl/mapl"));
    }

    #[test]
    fn rel_to_strips_root_prefix() {
        let out = rel_to(Path::new("/project"), Path::new("/project/.napl/map.json"));
        assert_eq!(out, ".napl/map.json");
    }

    #[test]
    fn rel_to_returns_path_unchanged_when_not_under_root() {
        let out = rel_to(Path::new("/project"), Path::new("/other/file.txt"));
        assert_eq!(out, "/other/file.txt");
    }

    #[test]
    fn rel_to_renders_forward_slashes() {
        let out = rel_to(Path::new("/root"), Path::new("/root/a/b/c/d.txt"));
        assert_eq!(out, "a/b/c/d.txt");
        assert!(!out.contains('\\'));
    }
}
