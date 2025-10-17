use sysinfo::System;

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub pid: u32,
    pub name: String,
}

impl ProcInfo {
    pub fn new(pid: u32, name: String) -> Self {
        ProcInfo { pid, name }
    }
}

pub fn get_list(filter: Option<&str>) -> Vec<ProcInfo> {
    let sys = System::new_all();
    let filter = filter.unwrap_or("");
    let f = filter.trim().to_lowercase();
    let mut proc_list = sys
        .processes()
        .iter()
        .filter_map(|(k, v)| {
            let name = v.name().to_str().unwrap_or("").to_owned();
            let pid = k.as_u32();
            if f.is_empty() || name.to_lowercase().starts_with(&f) {
                return Some(ProcInfo::new(pid, name));
            }

            None
        })
        .collect();

    if f.is_empty() {
        return proc_list;
    }

    proc_list.sort_by(|a, b| a.name.len().cmp(&b.name.len()));
    proc_list
}

mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_get_list_filtering() {
        let list = get_list(None);
        assert!(list.len() > 0);
        let list = get_list(Some("car"));

        for proc in list {
            assert!(proc.name.to_lowercase().starts_with("car"));
        }
    }
}
