//! The programs installed on this machine, as the Start menu lists them: desktop programs and
//! Store apps alike. The settings window offers them when an item is added.

use windows::core::{GUID, HSTRING, PWSTR};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{
    IEnumShellItems, IShellItem, IShellItem2, SHCreateItemFromParsingName, BHID_EnumItems, SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING,
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

#[cfg(test)]
mod tests {
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
