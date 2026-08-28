use std::path::{Path, PathBuf};
use std::sync::Mutex;

static PENDING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

pub fn register(path: PathBuf) {
    if let Ok(mut pending) = PENDING.lock() {
        pending.push(path);
    }
}

pub fn unregister(path: &Path) {
    if let Ok(mut pending) = PENDING.lock() {
        pending.retain(|p| p != path);
    }
}

pub fn run_now() {
    let paths = match PENDING.lock() {
        Ok(mut pending) => std::mem::take(&mut *pending),
        Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
    };
    for path in paths {
        let _ = std::fs::remove_file(&path);
    }
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

    static SERIAL: Mutex<()> = Mutex::new(());

    fn serial() -> std::sync::MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn a_registered_path_is_deleted_by_run_now() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret");
        std::fs::write(&path, "token").unwrap();

        register(path.clone());
        run_now();

        assert!(!path.exists());
    }

    #[test]
    fn an_unregistered_path_survives() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keep");
        std::fs::write(&path, "x").unwrap();

        register(path.clone());
        unregister(&path);
        run_now();

        assert!(
            path.exists(),
            "a guard that cleaned up normally must not have the hook delete a \
             file something else has since put back at the same path"
        );
    }

    #[test]
    fn run_now_empties_the_list_so_a_second_call_does_nothing() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("once");
        std::fs::write(&path, "x").unwrap();

        register(path.clone());
        run_now();
        std::fs::write(&path, "recreated by something else").unwrap();
        run_now();

        assert!(
            path.exists(),
            "draining the list is what stops a stale entry deleting a later file"
        );
    }

    #[test]
    fn a_missing_path_is_not_an_error() {
        let _serial = serial();
        register(PathBuf::from("definitely/not/here"));
        run_now();
    }
}
