// [INPUT]: 新旧应用数据目录（%APPDATA% 下 io.github.shiqiyux-cmd.cc-guard 与 local.proxy.ccbaga）、旧程序是否在运行。
// [OUTPUT]: 改名后首次启动把旧数据整份复制（不移动）到新目录，并按本用户独占权限重建每个目录和文件。
// [POS]: 启动最早一步；旧目录只读、一字不动；条件不满足或任何一步失败都不留下半份新目录。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use crate::identities::private;
use std::{fs, io, path::Path};

pub const OLD_DIR: &str = "local.proxy.ccbaga";
pub const OLD_EXE: &str = "requirement-french.exe";

/// 新目录不存在、旧目录存在且旧程序未运行时才复制；返回是否复制了。
/// 查不清旧程序是否在运行时按「在运行」处理，不冒险复制正在写入的数据。
pub fn copy_once(new: &Path, old: &Path, old_running: impl FnOnce() -> Result<bool, String>) -> Result<bool, String> {
    if fs::symlink_metadata(new).is_ok() {
        return Ok(false);
    }
    let Ok(meta) = fs::symlink_metadata(old) else { return Ok(false) };
    if !meta.is_dir() || crate::cleaner::linked(&meta) {
        return Ok(false);
    }
    if old_running().unwrap_or(true) {
        return Ok(false);
    }
    let name = new.file_name().ok_or("新数据目录无效。")?.to_string_lossy().into_owned();
    let staging = new.with_file_name(format!(".{name}.migrating-{}", crate::identities::new_id()?));
    let result = copy_tree(old, &staging).and_then(|_| fs::rename(&staging, new).map_err(|_| "无法启用迁移后的数据目录。".into()));
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result.map(|_| true)
}

/// 逐项复制；遇到链接或特殊文件整体放弃（不跟随，也不留下缺项的副本）。
fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    private::directory(to)?;
    for entry in fs::read_dir(from).map_err(|_| format!("无法读取旧数据目录 {}。", from.display()))? {
        let entry = entry.map_err(|_| "无法读取旧数据目录。")?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        let meta = fs::symlink_metadata(&source).map_err(|_| format!("无法读取 {}。", source.display()))?;
        if crate::cleaner::linked(&meta) {
            return Err(format!("旧数据里有链接 {}，已停止迁移。", source.display()));
        }
        if meta.is_dir() {
            copy_tree(&source, &target)?;
        } else if meta.is_file() {
            let mut input = fs::File::open(&source).map_err(|_| format!("无法读取 {}。", source.display()))?;
            let mut output = private::create(&target)?;
            io::copy(&mut input, &mut output)
                .and_then(|_| output.sync_all())
                .map_err(|_| format!("无法复制 {}。", source.display()))?;
        } else {
            return Err(format!("旧数据里有特殊文件 {}，已停止迁移。", source.display()));
        }
    }
    Ok(())
}

/// 启动时调用；失败只记录，旧目录保持原样，可关闭程序、删除新目录后重试。
pub fn run(app: &tauri::AppHandle) {
    use tauri::Manager;
    let Ok(new) = app.path().app_data_dir() else { return };
    let Some(old) = new.parent().map(|parent| parent.join(OLD_DIR)) else { return };
    if let Err(error) = copy_once(&new, &old, || crate::windows::image_running(OLD_EXE)) {
        eprintln!("旧数据迁移未完成：{error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp() -> PathBuf {
        let dir = dunce::canonicalize(std::env::temp_dir())
            .unwrap()
            .join(format!("cc-guard-migrate-{}", crate::identities::new_id().unwrap()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
    fn old_tree(root: &Path) -> PathBuf {
        let old = root.join(OLD_DIR);
        fs::create_dir_all(old.join("identity-vault/archives")).unwrap();
        fs::write(old.join("identity-vault/key.bin"), [7u8; 32]).unwrap();
        fs::write(old.join("identity-vault/archives/a.bagaid"), vec![1u8; 300_000]).unwrap();
        fs::write(old.join("settings.json"), br#"{"line":{"port":7899}}"#).unwrap();
        old
    }
    /// 目录树的完整指纹：相对路径、大小、修改时间与内容。
    fn fingerprint(root: &Path) -> Vec<(String, u64, std::time::SystemTime, Vec<u8>)> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let mut entries: Vec<_> = fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
            entries.sort();
            for path in entries {
                let meta = fs::metadata(&path).unwrap();
                let rel = path.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                if meta.is_dir() {
                    out.push((rel, 0, meta.modified().unwrap(), Vec::new()));
                    stack.push(path);
                } else {
                    out.push((rel, meta.len(), meta.modified().unwrap(), fs::read(&path).unwrap()));
                }
            }
        }
        out.sort();
        out
    }
    fn contents(root: &Path) -> Vec<(String, Vec<u8>)> {
        fingerprint(root).into_iter().map(|(rel, _, _, bytes)| (rel, bytes)).collect()
    }

    #[test]
    fn copies_everything_and_leaves_the_old_directory_untouched() {
        let root = temp();
        let old = old_tree(&root);
        let before = fingerprint(&old);
        let new = root.join("io.github.shiqiyux-cmd.cc-guard");
        assert!(copy_once(&new, &old, || Ok(false)).unwrap());
        assert_eq!(fingerprint(&old), before, "旧目录必须一字不动");
        assert_eq!(contents(&new), contents(&old));
        // 每个复制出来的目录和文件都重设为本用户所有，且没有别的账户条目
        for path in [new.clone(), new.join("identity-vault"), new.join("identity-vault/key.bin"), new.join("settings.json")] {
            assert!(crate::windows::owned(&path), "{}", path.display());
            let acl = std::process::Command::new("icacls").arg(&path).output().unwrap();
            let acl = String::from_utf8_lossy(&acl.stdout).to_lowercase();
            assert!(acl.contains(&std::env::var("USERNAME").unwrap().to_lowercase()), "{acl}");
            for other in ["everyone", "users:", "authenticated users", "(i)"] {
                assert!(!acl.contains(other), "{} 的权限含 {other}：{acl}", path.display());
            }
        }
        assert!(fs::read_dir(&root).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().contains("migrating")));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn skips_when_new_exists_old_missing_or_old_program_runs() {
        let root = temp();
        let old = old_tree(&root);
        let new = root.join("new");
        // 旧程序在运行，或查不清是否在运行：都不复制
        assert!(!copy_once(&new, &old, || Ok(true)).unwrap());
        assert!(!copy_once(&new, &old, || Err("查不了".into())).unwrap());
        assert!(!new.exists());
        // 旧目录不存在
        assert!(!copy_once(&new, &root.join("missing"), || Ok(false)).unwrap());
        assert!(!new.exists());
        // 新目录已存在：不覆盖、不合并
        fs::create_dir(&new).unwrap();
        fs::write(new.join("mine"), b"keep").unwrap();
        assert!(!copy_once(&new, &old, || Ok(false)).unwrap());
        assert_eq!(fs::read_dir(&new).unwrap().count(), 1);
        assert_eq!(fs::read(new.join("mine")).unwrap(), b"keep");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_link_inside_old_data_aborts_without_a_partial_copy() {
        let root = temp();
        let old = old_tree(&root);
        let outside = root.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("secret"), b"outside").unwrap();
        let link = old.join("browser");
        assert!(std::process::Command::new("cmd.exe")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(&link)
            .arg(&outside)
            .output()
            .unwrap()
            .status
            .success());
        let before = fingerprint(&outside);
        let new = root.join("new");
        assert!(copy_once(&new, &old, || Ok(false)).is_err());
        assert!(!new.exists());
        assert!(fs::read_dir(&root).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().contains("migrating")));
        assert_eq!(fingerprint(&outside), before);
        fs::remove_dir(&link).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn old_program_detection_is_exact_and_read_only() {
        let me = std::env::current_exe().unwrap().file_name().unwrap().to_string_lossy().into_owned();
        assert!(crate::windows::image_running(&me).unwrap());
        assert!(crate::windows::image_running(&me.to_uppercase()).unwrap());
        assert!(!crate::windows::image_running(&me[1..]).unwrap(), "不许按片段匹配");
        assert!(!crate::windows::image_running("cc-guard-no-such-program.exe").unwrap());
    }
}
