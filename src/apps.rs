//! The programs installed on this machine, as the Start menu lists them: desktop programs and
//! Store apps alike; and the ones pinned to the taskbar. The settings window offers them when an
//! item is added.

use windows::core::{GUID, HSTRING, PWSTR};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::System::Com::{
    CoCreateInstance, CoTaskMemFree, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, STGM_READ,
};
use windows::Win32::UI::Shell::{
    IEnumShellItems, IShellItem, IShellItem2, IShellLinkW, ILFree, SHCreateItemFromIDList, SHCreateItemFromParsingName, ShellLink, BHID_EnumItems,
    SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING,
};
use windows::core::Interface;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct App {
    pub name: String,
    /// What an `open` item is given to start it: the path of its exe where there is one and it
    /// takes no arguments, otherwise its `shell:AppsFolder\…` address.
    pub target: String,
    /// For a program opened by its exe: the id it gives its windows, when it gives one of its own.
    /// Its taskbar button shows that id instead of the exe, which is what a meter of it looks for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    /// What its shortcut passes to the program it starts: tells an uninstaller or an updater apart.
    #[serde(skip)]
    pub args: String,
}

/// An app's id as the shell names it, when it is one: an id that is a path is the exe's own, not
/// one the program gave itself.
fn own_app_id(id: &str) -> Option<String> {
    let id = id.trim();
    (!id.is_empty() && !id.contains(['\\', '/'])).then(|| id.to_string())
}

/// System.Link.TargetParsingPath: the file a Start menu entry starts.
const LINK_TARGET: PROPERTYKEY = PROPERTYKEY { fmtid: GUID::from_u128(0xb9b4b3fc_2b51_4a42_b5d8_324146afcf25), pid: 2 };
/// System.Link.Arguments: what it passes to that file.
const LINK_ARGUMENTS: PROPERTYKEY = PROPERTYKEY { fmtid: GUID::from_u128(0x436f2667_14e2_4feb_b30a_146c53b5b674), pid: 100 };

/// Takes over a string the shell allocated.
unsafe fn owned(text: PWSTR) -> String {
    let s = text.to_string().unwrap_or_default();
    CoTaskMemFree(Some(text.0 as *const _));
    s
}

/// Every entry of the Start menu's list of apps, by name.
pub fn installed() -> Vec<App> {
    let mut found = Vec::new();
    unsafe {
        // The shell needs COM on this thread, until its objects here are gone
        let _com = crate::win::Com::new(COINIT_APARTMENTTHREADED);
        let Ok(folder) = SHCreateItemFromParsingName::<_, _, IShellItem>(&HSTRING::from("shell:AppsFolder"), None) else { return found };
        let Ok(items) = folder.BindToHandler::<_, IEnumShellItems>(None, &BHID_EnumItems) else { return found };
        loop {
            let mut next = [None];
            let mut got = 0u32;
            if items.Next(&mut next, Some(&mut got)).is_err() || got == 0 {
                break;
            }
            let Some(item) = next[0].take() else { break };
            let (Ok(name), Ok(id)) = (item.GetDisplayName(SIGDN_NORMALDISPLAY), item.GetDisplayName(SIGDN_PARENTRELATIVEPARSING)) else { continue };
            let (name, id) = (owned(name), owned(id));
            if name.trim().is_empty() || id.is_empty() {
                continue;
            }
            // A plain desktop program is opened by its exe, so the panel can tell when it runs.
            // One started with arguments (a site installed as an app, say) keeps its address.
            let props = item.cast::<IShellItem2>().ok();
            let args = props.as_ref().and_then(|p| p.GetString(&LINK_ARGUMENTS).ok()).map(|a| owned(a)).unwrap_or_default();
            let exe = props.and_then(|props| {
                let path = props.GetString(&LINK_TARGET).ok().map(|p| owned(p))?;
                (args.trim().is_empty() && path.to_lowercase().ends_with(".exe") && std::path::Path::new(&path).is_file()).then_some(path)
            });
            let app = exe.as_ref().and_then(|_| own_app_id(&id));
            found.push(App { name, target: exe.unwrap_or_else(|| format!("shell:AppsFolder\\{id}")), app, args });
        }
    }
    found.sort_by_key(|a| a.name.to_lowercase());
    found.dedup_by(|a, b| a.target == b.target);
    found
}

/// System.AppUserModel.ID: the id of the app a shortcut stands for, when it says.
const APP_ID: PROPERTYKEY = PROPERTYKEY { fmtid: GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3), pid: 5 };
/// How the shell names an app of the Start menu's list of apps: this, then the app's id.
const APPS_FOLDER: &str = "::{4234D49B-0245-4DF3-B780-3893943456E1}\\";

/// What an `open` item is given for a shortcut, from what the shortcut holds: `place`, the shell's
/// name for what it points at (a file's path, an app of the Start menu's list, another shell
/// folder); the arguments it passes; the id of the app it stands for, if it says. The exe's path
/// when it takes no arguments, so the panel can tell when it runs; otherwise the app's
/// `shell:AppsFolder\…` address. `None` when it is neither.
fn link_target(place: &str, args: &str, app_id: Option<&str>) -> Option<String> {
    let app = |id: &str| (!id.trim().is_empty() && !id.contains('\\')).then(|| format!("shell:AppsFolder\\{}", id.trim()));
    if place.get(..APPS_FOLDER.len()).is_some_and(|head| head.eq_ignore_ascii_case(APPS_FOLDER)) {
        return app(&place[APPS_FOLDER.len()..]);
    }
    if place.to_lowercase().ends_with(".exe") && args.trim().is_empty() {
        return Some(place.to_string());
    }
    app_id.and_then(app)
}

/// Whether an exe or an app a shortcut names is on this machine: a program that is gone, or an id
/// no installed app has, is not offered.
fn opens(target: &str) -> bool {
    if target.starts_with("shell:") {
        unsafe { SHCreateItemFromParsingName::<_, _, IShellItem>(&HSTRING::from(target), None).is_ok() }
    } else {
        std::path::Path::new(target).is_file()
    }
}

/// Whether what an `open` item from somebody else's file opens is on this machine: a file or a
/// folder on a drive, or an app of the Start menu's list. An address (a site, `ms-settings:`, any
/// other link) counts as there. So does what cannot be told without going out for it: a network
/// path, which is never touched for a file that came from anywhere, and a bare name Windows
/// finds by itself.
pub fn found(target: &str) -> bool {
    let target = target.trim();
    let lower = target.to_lowercase();
    if lower.starts_with("shell:appsfolder\\") {
        // The shell needs COM on this thread
        let _com = crate::win::Com::new(COINIT_APARTMENTTHREADED);
        return opens(target);
    }
    let on_drive = matches!(lower.as_bytes(), [drive, b':', b'\\' | b'/', ..] if drive.is_ascii_lowercase());
    !on_drive || std::path::Path::new(target).exists()
}

/// The shortcuts pinned to the taskbar of this user, by name, each with what it starts. A shortcut
/// whose target cannot be told is left out.
pub fn pinned() -> Vec<App> {
    let Ok(appdata) = std::env::var("APPDATA") else { return Vec::new() };
    shortcuts_in(&std::path::Path::new(&appdata).join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar"))
}

/// The shortcuts in a folder, by name, as `pinned` lists them.
fn shortcuts_in(folder: &std::path::Path) -> Vec<App> {
    let Ok(files) = std::fs::read_dir(folder) else { return Vec::new() };
    let mut found = Vec::new();
    unsafe {
        // The shell needs COM on this thread, until its objects here are gone
        let _com = crate::win::Com::new(COINIT_APARTMENTTHREADED);
        for file in files.flatten().map(|f| f.path()) {
            if !file.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk")) {
                continue;
            }
            let path = HSTRING::from(file.as_os_str());
            let Ok(link) = CoCreateInstance::<_, IShellLinkW>(&ShellLink, None, CLSCTX_INPROC_SERVER) else { continue };
            let Ok(shortcut) = SHCreateItemFromParsingName::<_, _, IShellItem2>(&path, None) else { continue };
            if link.cast::<IPersistFile>().and_then(|f| f.Load(&path, STGM_READ)).is_err() {
                continue;
            }
            let Ok(list) = link.GetIDList() else { continue };
            let place = SHCreateItemFromIDList::<IShellItem>(list).and_then(|item| item.GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING));
            ILFree(Some(list));
            let Ok(place) = place.map(|p| owned(p)) else { continue };
            let mut args = [0u16; 1024];
            let _ = link.GetArguments(&mut args);
            let args = String::from_utf16_lossy(&args[..args.iter().position(|&c| c == 0).unwrap_or(args.len())]);
            let app_id = shortcut.GetString(&APP_ID).ok().map(|id| owned(id));
            let Some(target) = link_target(&place, &args, app_id.as_deref()) else { continue };
            if !opens(&target) {
                continue;
            }
            // The name Explorer shows for the shortcut, in the language of Windows
            let name = shortcut.GetDisplayName(SIGDN_NORMALDISPLAY).ok().map(|n| owned(n)).filter(|n| !n.trim().is_empty());
            let name = name.unwrap_or_else(|| file.file_stem().unwrap_or_default().to_string_lossy().into_owned());
            let app = if target.starts_with("shell:") { None } else { app_id.as_deref().and_then(own_app_id) };
            found.push(App { name, target, app, args });
        }
    }
    found.sort_by_key(|a| a.name.to_lowercase());
    found.dedup_by(|a, b| a.target == b.target);
    found
}

/// The card of programs, in the welcome window and from the Items page, offers this many at most.
pub const FIRST_CARD: usize = 8;

/// Whether a target is one of Windows' own programs the panel has a ready action for: File Explorer
/// and Settings. The card of programs does not offer them as programs.
fn has_default_key(target: &str) -> bool {
    let target = target.trim().to_lowercase();
    let file = target.rsplit(['\\', '/']).next().unwrap_or("");
    let app = target.strip_prefix("shell:appsfolder\\").unwrap_or("");
    matches!(file, "explorer.exe" | "systemsettings.exe") || app == "microsoft.windows.explorer" || app.starts_with("windows.immersivecontrolpanel_")
}

/// Whether a shortcut starts a program's service rather than the program: an uninstaller, an
/// updater, a setup, a helper, a crash reporter, or the Store itself. Told by the file name of the
/// exe it starts and by the arguments it passes, never by the name of a product.
fn service(app: &App) -> bool {
    let target = app.target.trim().to_lowercase();
    let store = target.strip_prefix("shell:appsfolder\\").is_some_and(|id| id.starts_with("microsoft.windowsstore_"));
    let file = target.rsplit(['\\', '/']).next().unwrap_or("");
    let exe = file.strip_suffix(".exe").filter(|_| file.ends_with(".exe"));
    let by_name = exe.is_some_and(|stem| {
        stem.starts_with("unins") || stem.starts_with("setup") || ["uninstall", "update", "helper", "crash"].iter().any(|word| stem.contains(word))
    });
    let args = app.args.to_lowercase();
    let by_args = args.split(|c: char| c.is_whitespace() || c == '=').any(|arg| {
        let arg = arg.trim_start_matches(['-', '/']);
        arg == "uninstall" || arg == "update" || arg == "x"
    });
    store || by_name || by_args
}

/// The programs of the windows open now, as the Start menu lists them, front to back and each once:
/// `windows` gives each window's program by the full path of its exe and the id of the app it stands
/// for. Matched as a meter matches a window (`meters::window_of`): an exe by its full path or the id
/// it gives its windows, an app by its whole id.
pub fn of_windows(installed: &[App], windows: &[(String, Option<String>)]) -> Vec<App> {
    let mut found: Vec<App> = Vec::new();
    for (path, id) in windows {
        let app = installed.iter().find(|a| crate::meters::window_of(&a.target, a.app.as_deref(), path, id.as_deref()));
        if let Some(app) = app.filter(|a| !found.iter().any(|f| f.target == a.target)) {
            found.push(app.clone());
        }
    }
    found
}

/// What the card of programs offers: the programs pinned to the taskbar, then those with windows
/// open now, `FIRST_CARD` at most, each once. Left out: the panel itself (`own`, the file name of
/// its exe), Windows' own programs that have a ready action, services (`service`), and the
/// programs `items` (the targets of the items) already open, told apart as the meters tell programs
/// apart.
pub fn first_card(pinned: Vec<App>, running: Vec<App>, items: &[String], own: &str) -> Vec<App> {
    let own = own.to_lowercase();
    let mut chosen: Vec<App> = Vec::new();
    for app in pinned.into_iter().chain(running) {
        if chosen.len() == FIRST_CARD {
            break;
        }
        let same = |target: &str| crate::meters::same_program(&app.target, target);
        let file = app.target.trim().rsplit(['\\', '/']).next().unwrap_or("").to_lowercase();
        if file == own || has_default_key(&app.target) || service(&app) || items.iter().any(|t| same(t)) || chosen.iter().any(|c| same(&c.target)) {
            continue;
        }
        chosen.push(app);
    }
    chosen
}

/// What a person calls a program with windows: its name in the Start menu, found by its file
/// name or its Store package family; otherwise its file name without `.exe`.
pub fn name_of(apps: &[App], exe: &str, family: Option<&str>) -> String {
    match apps.iter().find(|a| crate::actions::belongs(&a.target, exe, family)) {
        Some(app) => app.name.clone(),
        None if exe.is_empty() => family.unwrap_or_default().to_string(),
        None => exe.strip_suffix(".exe").unwrap_or(exe).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_id_a_program_gave_itself_is_kept_for_its_meter() {
        assert_eq!(own_app_id(" Vendor.Render "), Some("Vendor.Render".into()));
        assert_eq!(own_app_id("{6D809377-6AF0-444B-8957-A3773F02200E}\\Tool\\tool.exe"), None);
        assert_eq!(own_app_id("C:/Tools/tool.exe"), None);
        assert_eq!(own_app_id(""), None);
    }

    #[test]
    fn a_program_is_called_by_its_start_menu_name() {
        let apps = [
            App { name: "Paint Studio".into(), target: r"C:\Tools\Paint.exe".into(), ..App::default() },
            App { name: "Notes".into(), target: r"shell:AppsFolder\Vendor.Notes_abc123xyz!App".into(), ..App::default() },
        ];
        assert_eq!(name_of(&apps, "paint.exe", None), "Paint Studio");
        assert_eq!(name_of(&apps, "notes.exe", Some("Vendor.Notes_abc123xyz")), "Notes");
        assert_eq!(name_of(&apps, "", Some("vendor.notes_abc123xyz")), "Notes");
        // Not in the Start menu: the file name without its extension
        assert_eq!(name_of(&apps, "sketch.exe", None), "sketch");
        assert_eq!(name_of(&apps, "", Some("vendor.other_abc")), "vendor.other_abc");
    }

    #[test]
    fn the_first_card_offers_pinned_then_open_programs_without_the_ones_already_there() {
        let app = |name: &str, target: &str| App { name: name.into(), target: target.into(), ..App::default() };
        let pinned = vec![
            app("Files", r"shell:AppsFolder\Microsoft.Windows.Explorer"),
            app("Paint", r"C:\Tools\Paint.exe"),
            app("Tapka", r"C:\Program Files\Tapka\tapka.exe"),
            app("Notes", r"shell:AppsFolder\Vendor.Notes_abc!App"),
            app("Old", r"C:\Old\old.exe"),
        ];
        let running = vec![
            app("Settings", r"shell:AppsFolder\windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel"),
            app("Paint again", r"c:/tools/paint.EXE"),
            app("Render", r"D:\Render\render.exe"),
            app("Explorer", r"C:\Windows\explorer.exe"),
            app("Paint elsewhere", r"D:\Other\paint.exe"),
        ];
        // An item already opens Old, written another way
        let items = vec![r"c:\old\OLD.exe".to_string(), "ms-settings:".to_string()];
        let names = |found: Vec<App>| found.into_iter().map(|a| a.name).collect::<Vec<_>>();
        assert_eq!(names(first_card(pinned, running, &items, "tapka.exe")), ["Paint", "Notes", "Render", "Paint elsewhere"]);
        // No more than eight, pinned first
        let many: Vec<App> = (0..12).map(|i| app(&format!("P{i}"), &format!(r"C:\P{i}\p{i}.exe"))).collect();
        assert_eq!(names(first_card(many[..6].to_vec(), many[6..].to_vec(), &[], "tapka.exe")), ["P0", "P1", "P2", "P3", "P4", "P5", "P6", "P7"]);
        // Nothing found, nothing offered
        assert!(first_card(Vec::new(), Vec::new(), &items, "tapka.exe").is_empty());
    }

    #[test]
    fn the_first_card_leaves_out_uninstallers_updaters_helpers_and_the_store_before_it_counts_eight() {
        let app = |name: &str, target: &str, args: &str| App { name: name.into(), target: target.into(), args: args.into(), ..App::default() };
        let services = vec![
            app("Uninstall", r"C:\Tools\unins000.exe", ""),
            app("Remove", r"C:\Tools\Uninstall Tool.exe", ""),
            app("Update", r"C:\Users\Public\Tool\Update.exe", ""),
            app("Updater", r"C:\Tools\ToolUpdater.exe", ""),
            app("Helper", r"C:\Tools\tool_helper.exe", ""),
            app("Crash", r"C:\Tools\crashpad_handler.exe", ""),
            app("Reporter", r"C:\Tools\CrashHandler.exe", ""),
            app("Setup", r"C:\Tools\setup64.exe", ""),
            app("Remove by argument", r"shell:AppsFolder\Vendor.Tool", "--uninstall"),
            app("Update by argument", r"shell:AppsFolder\Vendor.Other", "/update now"),
            app("Installer", r"shell:AppsFolder\Vendor.Msi", "/x {0000}"),
            app("Store", r"shell:AppsFolder\Microsoft.WindowsStore_8wekyb3d8bbwe!App", ""),
        ];
        assert!(services.iter().all(service), "{:?}", services.iter().filter(|a| !service(a)).collect::<Vec<_>>());
        // A program started through an updater with an argument of its own, and plain programs
        assert!(!service(&app("Chat", r"shell:AppsFolder\Vendor.Chat", "--processStart chat.exe")));
        assert!(!service(&app("Paint", r"C:\Tools\paint.exe", "")));
        assert!(!service(&app("Notes", r"shell:AppsFolder\Vendor.Notes_abc!App", "")));
        // The services do not take places among the eight
        let many: Vec<App> = (0..9).map(|i| app(&format!("P{i}"), &format!(r"C:\P{i}\p{i}.exe"), "")).collect();
        let found = first_card(services.into_iter().chain(many[..4].to_vec()).collect(), many[4..].to_vec(), &[], "tapka.exe");
        assert_eq!(found.into_iter().map(|a| a.name).collect::<Vec<_>>(), ["P0", "P1", "P2", "P3", "P4", "P5", "P6", "P7"]);
    }

    #[test]
    fn an_open_window_is_its_program_by_full_path_or_whole_app_id() {
        let app = |name: &str, target: &str, id: Option<&str>| App { name: name.into(), target: target.into(), app: id.map(Into::into), ..App::default() };
        let installed = [
            app("Old Paint", r"D:\Old\paint.exe", None),
            app("Paint", r"C:\Tools\paint.exe", None),
            app("Player", r"shell:AppsFolder\Vendor.Media_abc!Player", None),
            app("Editor", r"shell:AppsFolder\Vendor.Media_abc!Editor", None),
            app("Render", r"C:\Render\render.exe", Some("Vendor.Render")),
        ];
        let window = |path: &str, id: Option<&str>| (path.to_string(), id.map(Into::into));
        let windows = [
            window(r"c:\tools\PAINT.exe", None),
            window(r"C:\Program Files\WindowsApps\media.exe", Some("vendor.media_abc!editor")),
            window(r"C:\Tools\paint.exe", None),
            window(r"C:\Render\render-gpu.exe", Some("Vendor.Render")),
            window(r"E:\Elsewhere\paint.exe", None),
        ];
        let names: Vec<String> = of_windows(&installed, &windows).into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["Paint", "Editor", "Render"]);
    }

    #[test]
    fn a_pinned_shortcut_opens_its_exe_or_its_app() {
        // A program without arguments is opened by its exe, even when the shortcut names its app
        assert_eq!(link_target(r"C:\Tools\paint.exe", "", None).as_deref(), Some(r"C:\Tools\paint.exe"));
        assert_eq!(link_target(r"C:\Tools\Paint.EXE", "  ", Some("Vendor.Paint")).as_deref(), Some(r"C:\Tools\Paint.EXE"));
        // A Store app by its place in the Start menu's list
        let app = r"::{4234d49b-0245-4df3-b780-3893943456e1}\Vendor.Paint_abc123xyz!App";
        assert_eq!(link_target(app, "", None).as_deref(), Some(r"shell:AppsFolder\Vendor.Paint_abc123xyz!App"));
        // A shell folder or a program with arguments, by the app the shortcut names
        let folder = "::{20D04FE0-3AEA-1069-A2D8-08002B30309D}";
        assert_eq!(link_target(folder, "", Some("Vendor.Files")).as_deref(), Some(r"shell:AppsFolder\Vendor.Files"));
        assert_eq!(link_target(r"C:\Tools\paint.exe", "--app=sketch", Some("Vendor.Sketch")).as_deref(), Some(r"shell:AppsFolder\Vendor.Sketch"));
        // Nothing to tell it by: arguments, a document, a folder, an empty app, a shell folder
        assert_eq!(link_target(r"C:\Tools\paint.exe", "--profile 2", None), None);
        assert_eq!(link_target(r"C:\Docs\notes.txt", "", None), None);
        assert_eq!(link_target(r"C:\Tools", "", Some(" ")), None);
        assert_eq!(link_target(r"::{4234D49B-0245-4DF3-B780-3893943456E1}\", "", Some("Vendor.Paint")), None);
        assert_eq!(link_target(folder, "", None), None);
    }

    #[test]
    fn only_a_program_or_an_app_that_is_there_is_offered() {
        let windir = std::env::var("WINDIR").unwrap();
        assert!(opens(&format!("{windir}\\explorer.exe")));
        assert!(!opens(r"C:\no\such\paint.exe"));
        // An app of Windows itself, and an id no app has; the shell needs COM, as in `pinned`
        let _com = crate::win::Com::new(COINIT_APARTMENTTHREADED);
        assert!(opens(r"shell:AppsFolder\Microsoft.Windows.Explorer"));
        assert!(!opens(r"shell:AppsFolder\Vendor.NoSuchPaint_abc123xyz!App"));
    }

    #[test]
    fn a_target_from_a_file_is_found_only_when_it_is_on_this_machine() {
        let windir = std::env::var("WINDIR").unwrap();
        // A file, a folder and an app that are here
        assert!(found(&format!("{windir}\\explorer.exe")));
        assert!(found(&format!(" {windir} ")));
        assert!(found(r"shell:AppsFolder\Microsoft.Windows.Explorer"));
        // The same that are not
        assert!(!found(r"C:\no\such\paint.exe"));
        assert!(!found("c:/no/such/folder"));
        assert!(!found(r"shell:AppsFolder\Vendor.NoSuchPaint_abc123xyz!App"));
        // Addresses count as found; a network path is not gone to
        for address in ["https://example.com", "ms-settings:sound", "ms-settings:", "chat://new", "notepad", r"\\no-such-host\share\a.txt"] {
            assert!(found(address), "{address}");
        }
    }

    #[test]
    fn a_pinned_shortcut_is_listed_by_its_name_with_the_program_it_opens() {
        let windir = std::env::var("WINDIR").unwrap();
        let explorer = format!("{windir}\\explorer.exe");
        let folder = std::env::temp_dir().join(format!("tapka-pinned-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        // A shortcut to a program, one to a program that is gone, and a file that is no shortcut
        let make = |name: &str, target: &str| unsafe {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
            link.SetPath(&HSTRING::from(target)).unwrap();
            link.cast::<IPersistFile>().unwrap().Save(&HSTRING::from(folder.join(name).as_os_str()), true).unwrap();
        };
        {
            let _com = crate::win::Com::new(COINIT_APARTMENTTHREADED);
            make("Files.lnk", &explorer);
            make("Gone.lnk", r"C:\no\such\paint.exe");
        }
        std::fs::write(folder.join("notes.txt"), "not a shortcut").unwrap();
        let found = shortcuts_in(&folder);
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "Files");
        assert!(found[0].target.eq_ignore_ascii_case(&explorer), "{:?}", found[0]);
        assert_eq!(found[0].app, None);
    }

    #[test]
    fn windows_has_programs_and_each_can_be_opened_and_drawn() {
        let apps = super::installed();
        assert!(apps.len() > 5, "{} programs", apps.len());
        assert!(apps.iter().all(|a| !a.name.trim().is_empty()));
        assert!(apps.iter().all(|a| a.target.starts_with("shell:AppsFolder\\") || a.target.to_lowercase().ends_with(".exe")));
        // Sorted by name, and the first of them has an icon to show
        assert!(apps.windows(2).all(|w| w[0].name.to_lowercase() <= w[1].name.to_lowercase()));
        assert!(crate::shellicon::png(&apps[0].target, 48).is_some(), "{:?}", apps[0]);
    }
}
