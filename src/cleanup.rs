use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Default)]
struct Registry {
    paths: Vec<PathBuf>,
}

impl Registry {
    fn register(&mut self, path: PathBuf) {
        self.paths.push(path);
    }

    fn unregister(&mut self, path: &Path) {
        self.paths.retain(|p| p != path);
    }

    fn drain_and_delete(&mut self) {
        for path in std::mem::take(&mut self.paths) {
            let _ = std::fs::remove_file(&path);
        }
    }
}

static PENDING: Mutex<Registry> = Mutex::new(Registry { paths: Vec::new() });

fn pending() -> std::sync::MutexGuard<'static, Registry> {
    PENDING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn register(path: PathBuf) {
    pending().register(path);
}

pub fn unregister(path: &Path) {
    pending().unregister(path);
}

pub fn run_now() {
    pending().drain_and_delete();
}

pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        run_now();
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_registered_path_is_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret");
        std::fs::write(&path, "token").unwrap();

        let mut registry = Registry::default();
        registry.register(path.clone());
        registry.drain_and_delete();

        assert!(!path.exists());
    }

    #[test]
    fn an_unregistered_path_survives() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keep");
        std::fs::write(&path, "x").unwrap();

        let mut registry = Registry::default();
        registry.register(path.clone());
        registry.unregister(&path);
        registry.drain_and_delete();

        assert!(
            path.exists(),
            "a guard that cleaned up normally must not have the hook delete a \
             file something else has since put back at the same path"
        );
    }

    #[test]
    fn draining_empties_the_list_so_a_second_pass_does_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("once");
        std::fs::write(&path, "x").unwrap();

        let mut registry = Registry::default();
        registry.register(path.clone());
        registry.drain_and_delete();
        std::fs::write(&path, "recreated by something else").unwrap();
        registry.drain_and_delete();

        assert!(
            path.exists(),
            "draining is what stops a stale entry deleting a later file"
        );
    }

    #[test]
    fn unregister_removes_only_the_named_path() {
        let dir = tempfile::tempdir().unwrap();
        let kept = dir.path().join("kept");
        let dropped = dir.path().join("dropped");
        std::fs::write(&kept, "x").unwrap();
        std::fs::write(&dropped, "x").unwrap();

        let mut registry = Registry::default();
        registry.register(kept.clone());
        registry.register(dropped.clone());
        registry.unregister(&kept);
        registry.drain_and_delete();

        assert!(kept.exists());
        assert!(!dropped.exists());
    }

    #[test]
    fn a_missing_path_is_not_an_error() {
        let mut registry = Registry::default();
        registry.register(PathBuf::from("definitely/not/here"));
        registry.drain_and_delete();
    }

    #[test]
    fn a_poisoned_lock_still_yields_the_registry() {
        let _ = std::panic::catch_unwind(|| {
            let _guard = pending();
            panic!("poison the global registry");
        });

        pending().register(PathBuf::from("still/reachable"));
        pending().unregister(Path::new("still/reachable"));
    }
}
