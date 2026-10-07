//! The programs installed on this machine, as the Start menu lists them: desktop programs and
//! Store apps alike; and the ones pinned to the taskbar. The settings window offers them when an
//! item is added.

use windows::core::{GUID, HSTRING, PWSTR};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, STGM_READ,
};
use windows::Win32::UI::Shell::{
    IEnumShellItems, IShellItem, IShellItem2, IShellLinkW, ILFree, SHCreateItemFromIDList, SHCreateItemFromParsingName, ShellLink, BHID_EnumItems,
    SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING,
};
use windows::core::Interface;

#[derive(Debug, Clone, serde::Serialize)]
pub struct App {
    pub name: String,
    /// What an `open` item is given to start it: the path of its exe where there is one and it
    /// takes no arguments, otherwise its `shell:AppsFolder\…` address.
    pub target: String,
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
        // The shell needs COM on this thread; a thread that already has it answers with an error
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
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
            let exe = item.cast::<IShellItem2>().ok().and_then(|props| {
                let path = props.GetString(&LINK_TARGET).ok().map(|p| owned(p))?;
                let plain = props.GetString(&LINK_ARGUMENTS).ok().map(|a| owned(a)).is_none_or(|a| a.trim().is_empty());
                (plain && path.to_lowercase().ends_with(".exe") && std::path::Path::new(&path).is_file()).then_some(path)
            });
            found.push(App { name, target: exe.unwrap_or_else(|| format!("shell:AppsFolder\\{id}")) });
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

/// The shortcuts pinned to the taskbar of this user, by name, each with what it starts. A shortcut
/// whose target cannot be told is left out.
pub fn pinned() -> Vec<App> {
    let Ok(appdata) = std::env::var("APPDATA") else { return Vec::new() };
    let folder = std::path::Path::new(&appdata).join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar");
    let Ok(files) = std::fs::read_dir(folder) else { return Vec::new() };
    let mut found = Vec::new();
    unsafe {
        // The shell needs COM on this thread; a thread that already has it answers with an error
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
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
            found.push(App { name, target });
        }
    }
    found.sort_by_key(|a| a.name.to_lowercase());
    found.dedup_by(|a, b| a.target == b.target);
    found
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
    fn a_program_is_called_by_its_start_menu_name() {
        let apps = [
            App { name: "Paint Studio".into(), target: r"C:\Tools\Paint.exe".into() },
            App { name: "Notes".into(), target: r"shell:AppsFolder\Vendor.Notes_abc123xyz!App".into() },
        ];
        assert_eq!(name_of(&apps, "paint.exe", None), "Paint Studio");
        assert_eq!(name_of(&apps, "notes.exe", Some("Vendor.Notes_abc123xyz")), "Notes");
        assert_eq!(name_of(&apps, "", Some("vendor.notes_abc123xyz")), "Notes");
        // Not in the Start menu: the file name without its extension
        assert_eq!(name_of(&apps, "sketch.exe", None), "sketch");
        assert_eq!(name_of(&apps, "", Some("vendor.other_abc")), "vendor.other_abc");
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
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        assert!(opens(r"shell:AppsFolder\Microsoft.Windows.Explorer"));
        assert!(!opens(r"shell:AppsFolder\Vendor.NoSuchPaint_abc123xyz!App"));
    }

    #[test]
    fn the_pinned_shortcuts_of_this_machine_can_be_opened() {
        let pinned = super::pinned();
        assert!(pinned.iter().all(|a| !a.name.trim().is_empty()));
        assert!(pinned.iter().all(|a| a.target.starts_with("shell:AppsFolder\\") || a.target.to_lowercase().ends_with(".exe")));
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
