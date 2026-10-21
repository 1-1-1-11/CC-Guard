// [INPUT]: line::gate (a fresh Claude line check), the shared gated launch, UI language and HKCU registration.
// [OUTPUT]: Tray icon/tooltip/menu showing the Claude line, one Windows notification per fault and recovery,
//           tray launches and start at sign-in (hidden in the tray).
// [POS]: Background face of the single entry; launches through the page's native gate.
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use crate::line::Entry;
use crate::mihomo::{Outcome, Verdict};
use std::{sync::Mutex, time::Duration};
use tauri::{image::Image, Emitter, Manager};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Health { Ready, Blocked, Unknown }

#[derive(Default)]
pub struct Tray(Mutex<Option<Health>>);

/// Line check → health; only a passing check is ready.
fn health(check: &Outcome) -> Health {
    match check.verdict { Verdict::Pass => Health::Ready, Verdict::Block => Health::Blocked, Verdict::Unknown => Health::Unknown }
}

/// A fault is announced after two samples in a row and once per kind; recovery only after an announced fault.
#[derive(Default)]
struct Notices { announced: Option<Health>, pending: bool }
impl Notices {
    fn next(&mut self, health: Health) -> Option<Health> {
        if health == Health::Ready {
            self.pending = false;
            return self.announced.take().map(|_| Health::Ready);
        }
        if !std::mem::replace(&mut self.pending, true) || self.announced == Some(health) { return None; }
        self.announced = Some(health);
        Some(health)
    }
}

fn english(app: &tauri::AppHandle) -> bool {
    app.state::<crate::ui_language::UiLanguage>().0.lock().map(|language| *language == "en").unwrap_or(false)
}
fn words(health: Option<Health>, en: bool) -> &'static str {
    match (health, en) {
        (Some(Health::Ready), false) => "Claude 线路正常",
        (Some(Health::Ready), true) => "Claude line is healthy",
        (Some(Health::Blocked), false) => "Claude 线路未通过，启动已暂停",
        (Some(Health::Blocked), true) => "Claude line failed; launches paused",
        (_, false) => "Claude 线路待确认",
        (_, true) => "Claude line unconfirmed",
    }
}

/// The app icon with a status dot: green ready, orange blocked, grey unconfirmed.
fn badge(app: &tauri::AppHandle, health: Option<Health>) -> Option<Image<'static>> {
    let icon = app.default_window_icon()?;
    let (width, height) = (icon.width(), icon.height());
    let mut rgba = icon.rgba().to_vec();
    let color = match health { Some(Health::Ready) => [46, 139, 63], Some(Health::Blocked) => [214, 96, 26], _ => [128, 128, 128] };
    let radius = width.min(height) as f32 * 0.24;
    let (cx, cy) = (width as f32 - radius - 1.0, height as f32 - radius - 1.0);
    for y in 0..height {
        for x in 0..width {
            let distance = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let pixel = ((y * width + x) * 4) as usize;
            if distance <= radius - radius * 0.22 { rgba[pixel..pixel + 4].copy_from_slice(&[color[0], color[1], color[2], 255]); }
            else if distance <= radius { rgba[pixel..pixel + 4].copy_from_slice(&[255, 255, 255, 255]); }
        }
    }
    Some(Image::new_owned(rgba, width, height))
}

fn menu(app: &tauri::AppHandle, health: Option<Health>) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
    let en = english(app);
    let pick = |zh: &'static str, english_text: &'static str| if en { english_text } else { zh };
    let item = |id: &str, text: &str, enabled: bool| MenuItem::with_id(app, id, text, enabled, None::<&str>);
    let status = item("status", words(health, en), false)?;
    let open = item("open", pick("打开 CC-Guard", "Open CC-Guard"), true)?;
    let cli = item("cli", pick("启动 Claude CLI", "Start Claude CLI"), true)?;
    let desktop = item("desktop", pick("启动 Claude 桌面客户端", "Start Claude Desktop"), true)?;
    let web = item("web", pick("打开网页版（指纹浏览器）", "Open web (fingerprint browser)"), true)?;
    let autostart = CheckMenuItem::with_id(app, "autostart", pick("开机后在托盘待命", "Start in tray at sign-in"), cfg!(windows), autostart::enabled(), None::<&str>)?;
    let quit = item("quit", pick("退出", "Quit"), true)?;
    let line = || PredefinedMenuItem::separator(app);
    Menu::with_items(app, &[&status, &open, &line()?, &cli, &desktop, &web, &line()?, &autostart, &line()?, &quit])
}

/// Repaints icon, tooltip and menu for the current health and UI language.
pub fn refresh(app: &tauri::AppHandle) {
    let health = app.try_state::<Tray>().and_then(|tray| tray.0.lock().ok().and_then(|state| *state));
    let Some(tray) = app.tray_by_id("main") else { return };
    if let Ok(menu) = menu(app, health) { let _ = tray.set_menu(Some(menu)); }
    let _ = tray.set_icon(badge(app, health));
    let _ = tray.set_tooltip(Some(format!("CC-Guard · {}", words(health, english(app)))));
}

pub fn build(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    app.manage(Tray::default());
    autostart::init(app);
    notify::register();
    TrayIconBuilder::with_id("main")
        .icon(badge(app, None).unwrap_or_else(|| app.default_window_icon().unwrap().clone()))
        .menu(&menu(app, None)?)
        .show_menu_on_left_click(false)
        .tooltip(format!("CC-Guard · {}", words(None, english(app))))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                crate::show(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => crate::show(app),
            "cli" => start(app, Entry::Cli),
            "desktop" => start(app, Entry::Desktop),
            "web" => start(app, Entry::Web),
            "autostart" => { let _ = autostart::set(!autostart::enabled()); refresh(app); }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

/// Same native gate as the page. Anything but a requested launch opens the window with the reason.
fn start(app: &tauri::AppHandle, entry: Entry) {
    let app = app.clone();
    std::thread::spawn(move || {
        let message = match crate::line::launch_blocking(&app, entry) {
            Ok(result) if result.launched => return,
            Ok(result) => result.reason,
            Err(error) => error,
        };
        crate::show(&app);
        let _ = app.emit("proxy-launch-blocked", message);
    });
}

/// Re-runs the line check every 5 minutes: each check queries two exit services through the
/// Claude-only port, so a shorter interval would exhaust their free quotas.
pub fn watch(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut notices = Notices::default();
        loop {
            let check = match crate::line::gate(&app) { Ok(gate) => gate.check, Err(check) => check };
            let health = health(&check);
            let changed = app.state::<Tray>().0.lock().map(|mut state| state.replace(health) != Some(health)).unwrap_or(false);
            if changed { refresh(&app); }
            if let Some(notice) = notices.next(health) {
                let en = english(&app);
                let body = match (notice, en) {
                    (Health::Ready, false) => "可以继续使用 Claude。",
                    (Health::Ready, true) => "Claude can be used again.",
                    (Health::Blocked, false) => "Claude 启动已暂停。打开 CC-Guard 查看原因。",
                    (Health::Blocked, true) => "Claude launches are paused. Open CC-Guard for the reason.",
                    (Health::Unknown, false) => "线路暂时无法确认，启动 Claude 前会重新检查。",
                    (Health::Unknown, true) => "The line can't be confirmed. Claude launches check again first.",
                };
                let title = match (notice, en) { (Health::Ready, false) => "Claude 线路已恢复", (Health::Ready, true) => "Claude line recovered", _ => words(Some(notice), en) };
                notify::show(title, body);
            }
            std::thread::sleep(Duration::from_secs(300));
        }
    });
}

mod registry {
    use windows_sys::Win32::System::Registry::*;
    fn wide(text: &str) -> Vec<u16> { text.encode_utf16().chain(Some(0)).collect() }
    pub fn read(path: &str, name: &str) -> Option<String> { crate::windows::registry_string(HKEY_CURRENT_USER, path, name) }
    pub fn write(path: &str, name: &str, value: &str) -> bool {
        let (path, name, value) = (wide(path), wide(name), wide(value));
        let mut key: HKEY = std::ptr::null_mut();
        unsafe {
            if RegCreateKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, std::ptr::null(), REG_OPTION_NON_VOLATILE,
                               KEY_SET_VALUE, std::ptr::null(), &mut key, std::ptr::null_mut()) != 0 { return false; }
            let written = RegSetValueExW(key, name.as_ptr(), 0, REG_SZ, value.as_ptr().cast(), (value.len() * 2) as u32) == 0;
            RegCloseKey(key);
            written
        }
    }
    pub fn remove(path: &str, name: &str) -> bool {
        let status = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, wide(path).as_ptr(), wide(name).as_ptr()) };
        status == 0 || status == 2
    }
}

/// HKCU Run entry for this executable, started hidden in the tray. On by default for the release build.
mod autostart {
    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const NAME: &str = "CC-Guard";
    fn command() -> Option<String> { std::env::current_exe().ok().map(|exe| format!("\"{}\" --tray", exe.display())) }
    pub fn enabled() -> bool {
        super::registry::read(RUN, NAME).is_some()
    }
    pub fn set(on: bool) -> Result<(), String> {
        {
            let done = match (on, command()) {
                (true, Some(command)) => super::registry::write(RUN, NAME, &command),
                (true, None) => false,
                (false, _) => super::registry::remove(RUN, NAME),
            };
            if done { Ok(()) } else { Err("无法修改开机启动。".into()) }
        }
    }
    /// First release start turns it on once; later starts only follow a moved executable.
    pub fn init(app: &tauri::AppHandle) {
        #[cfg(not(debug_assertions))]
        {
            use tauri::Manager;
            let Ok(dir) = app.path().app_data_dir() else { return };
            let marker = dir.join("autostart-initialized");
            if !marker.exists() {
                if set(true).is_ok() { let _ = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&marker, b"1")); }
            } else if let (Some(saved), Some(command)) = (super::registry::read(RUN, NAME), command()) {
                if saved != command { let _ = set(true); }
            }
        }
        let _ = app;
    }
}

/// Windows toast under this app's own AppUserModelID; registered per user so no shortcut is required.
mod notify {
    const AUMID: &str = "io.github.shiqiyux-cmd.cc-guard";
    pub fn register() {
        { let _ = super::registry::write(&format!(r"Software\Classes\AppUserModelId\{AUMID}"), "DisplayName", "CC-Guard"); }
    }
    pub fn show(title: &str, body: &str) {
        {
            use windows::{core::HSTRING, Data::Xml::Dom::XmlDocument, UI::Notifications::{ToastNotification, ToastNotificationManager}};
            let escape = |text: &str| text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
            let _ = (|| -> windows::core::Result<()> {
                let xml = XmlDocument::new()?;
                xml.LoadXml(&HSTRING::from(format!("<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>", escape(title), escape(body))))?;
                ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(AUMID))?.Show(&ToastNotification::CreateToastNotification(&xml)?)
            })();
        }
    }
    #[cfg(test)]
    #[test]
    #[ignore = "shows a real Windows notification; run explicitly"]
    fn real_notification() {
        register();
        show("CC-Guard 测试通知", "这是一条测试通知，可以忽略。");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_a_passing_line_check_is_ready() {
        let of = |verdict| { let mut check = crate::mihomo::unconfirmed("样本".into()); check.verdict = verdict; health(&check) };
        assert_eq!(of(Verdict::Pass), Health::Ready);
        assert_eq!(of(Verdict::Block), Health::Blocked);
        assert_eq!(of(Verdict::Unknown), Health::Unknown);
    }
    #[test]
    fn one_odd_sample_is_quiet_and_each_fault_is_announced_once() {
        use Health::*;
        let mut notices = Notices::default();
        let seen: Vec<_> = [Ready, Blocked, Ready, Blocked, Blocked, Blocked, Unknown, Unknown, Ready, Ready, Unknown, Ready]
            .into_iter().map(|health| notices.next(health)).collect();
        assert_eq!(seen, [None, None, None, None, Some(Blocked), None, Some(Unknown), None, Some(Ready), None, None, None]);
    }
}
