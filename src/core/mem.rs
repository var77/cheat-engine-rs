use process_memory::*;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq)]
pub enum MemoryError {
    NoPermission(i32),
    MemRead(i32),
    MemWrite(i32),
    ProcessAttach(i32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryRegionPerms {
    Read,
    Write,
}

pub const DEFAULT_SEARCH_PERMS: [MemoryRegionPerms; 1] = [MemoryRegionPerms::Write];

impl Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPermission(code) => write!(f, "Permission Denied: OS Error ({code})"),
            Self::MemRead(code) => write!(f, "Could not read memory: OS Error ({code})"),
            Self::MemWrite(code) => write!(f, "Could not write memory: OS Error ({code})"),
            Self::ProcessAttach(code) => {
                write!(f, "Could not attach to process: OS Error ({code})")
            }
        }
    }
}

#[derive(Debug)]
pub struct MemoryRegion {
    pub start: u64,
    pub end: u64,
    #[allow(dead_code)]
    pub perms: Vec<MemoryRegionPerms>,
}

#[cfg(target_os = "macos")]
pub fn get_memory_regions(
    pid: u32,
    start: Option<u64>,
    end: Option<u64>,
    search_perms: Option<&[MemoryRegionPerms]>,
) -> Result<Vec<MemoryRegion>, MemoryError> {
    use mach_sys::{
        kern_return::{KERN_INVALID_ADDRESS, KERN_SUCCESS},
        port::mach_port_name_t,
        traps::{mach_task_self, task_for_pid},
        vm::mach_vm_region,
        vm_prot::{VM_PROT_READ, VM_PROT_WRITE},
        vm_region::{VM_REGION_BASIC_INFO_64, vm_region_info_t},
        vm_types::{mach_vm_address_t, mach_vm_size_t, vm_map_t},
    };
    use mach_sys::{port::mach_port_t, vm_region::vm_region_basic_info_data_64_t};

    let search_perms = search_perms.unwrap_or(&DEFAULT_SEARCH_PERMS);

    let task: mach_port_name_t = 0;
    let kret = unsafe {
        task_for_pid(
            mach_task_self() as mach_port_name_t,
            pid as i32,
            &task as *const u32 as *mut u32,
        )
    };

    if kret != KERN_SUCCESS {
        return Err(MemoryError::NoPermission(kret));
    }

    let mut regions = Vec::new();
    let mut address: mach_vm_address_t = start.unwrap_or(1);
    let end: mach_vm_address_t = end.unwrap_or(u64::MAX);
    let mut size: mach_vm_size_t = 0;

    loop {
        if address > end {
            break;
        }

        let mut info = vm_region_basic_info_data_64_t::default();
        let mut info_count = VM_REGION_BASIC_INFO_64 as u32;
        let mut object_name: mach_port_t = 0;
        let kr = unsafe {
            mach_vm_region(
                task as vm_map_t,
                &mut address,
                &mut size,
                VM_REGION_BASIC_INFO_64,
                (&mut info as *mut vm_region_basic_info_data_64_t) as vm_region_info_t,
                &mut info_count as *mut u32,
                &mut object_name,
            )
        };

        if kr == KERN_INVALID_ADDRESS {
            break;
        } else if kr != KERN_SUCCESS {
            return Err(MemoryError::MemRead(kr));
        }

        let mut perms = Vec::with_capacity(2);
        if info.protection & VM_PROT_READ != 0 {
            perms.push(MemoryRegionPerms::Read);
        }

        if info.protection & VM_PROT_WRITE != 0 {
            perms.push(MemoryRegionPerms::Write);
        }

        if search_perms.iter().filter(|p| perms.contains(p)).count() > 0 {
            regions.push(MemoryRegion {
                start: address,
                end: address + size,
                perms,
            });
        }

        address += size;
    }

    Ok(regions)
}

#[cfg(target_os = "linux")]
pub fn get_memory_regions(
    pid: u32,
    start: Option<u64>,
    end: Option<u64>,
    search_perms: Option<&[MemoryRegionPerms]>,
) -> Result<Vec<MemoryRegion>, MemoryError> {
    use std::fs::File;
    use std::io::{self, BufRead};
    use std::path::PathBuf;

    let search_perms = search_perms.unwrap_or(&DEFAULT_SEARCH_PERMS);
    let path = PathBuf::from(format!("/proc/{}/maps", pid));
    let file = File::open(&path)
        .map_err(|e| MemoryError::NoPermission(e.raw_os_error().unwrap_or(-1) as i32))?;
    let reader = io::BufReader::new(file);

    let start_addr = start.unwrap_or(0);
    let end_addr = end.unwrap_or(u64::MAX);

    let mut regions = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|_| MemoryError::MemRead(0))?;
        // 00400000-00452000 r-xp 00000000 fd:00
        let mut parts = line.split_whitespace();
        let range = parts.next().ok_or_else(|| MemoryError::MemRead(0))?;
        let perms = parts.next().unwrap_or("");

        let mut range_split = range.split('-');
        let start_str = range_split.next().ok_or_else(|| MemoryError::MemRead(0))?;
        let end_str = range_split.next().ok_or_else(|| MemoryError::MemRead(0))?;
        let start_addr_val =
            u64::from_str_radix(start_str, 16).map_err(|_| MemoryError::MemRead(0))?;
        let end_addr_val = u64::from_str_radix(end_str, 16).map_err(|_| MemoryError::MemRead(0))?;

        // Filter by address range
        if end_addr_val < start_addr || start_addr_val > end_addr {
            continue;
        }

        let mut region_perms = Vec::with_capacity(2);
        let perms = perms[..3];

        if perms.contains('r') {
            region_perms.push(MemoryRegionPerms::Read);
        }

        if perms.contains('w') {
            region_perms.push(MemoryRegionPerms::Write);
        }

        if search_perms
            .iter()
            .filter(|p| region_perms.contains(p))
            .count()
            > 0
        {
            regions.push(MemoryRegion {
                start: start_addr_val,
                end: end_addr_val,
                perms: region_perms,
            });
        }
    }

    Ok(regions)
}

pub fn read_memory_address(pid: u32, addr: usize, size: usize) -> Result<Vec<u8>, MemoryError> {
    let handle = (pid as Pid)
        .try_into_process_handle()
        .map_err(|e| MemoryError::ProcessAttach(e.raw_os_error().unwrap_or(-1)))?;

    let mut result = vec![0; size];
    handle.copy_address(addr, &mut result).map_err(|e| {
        // in linux it can attach to process, but not read the memory
        // so this is a 'hack' to make it like MacOS
        if std::env::consts::OS == "linux" && e.raw_os_error().unwrap_or(-1) == 1 {
            return MemoryError::ProcessAttach(1);
        }
        MemoryError::MemRead(e.raw_os_error().unwrap_or(-1))
    })?;

    Ok(result)
}

pub fn write_memory_address(pid: u32, addr: usize, value: &[u8]) -> Result<(), MemoryError> {
    let handle = (pid as Pid)
        .try_into_process_handle()
        .map_err(|e| MemoryError::ProcessAttach(e.raw_os_error().unwrap_or(-1)))?;

    handle
        .put_address(addr, value)
        .map_err(|e| MemoryError::MemWrite(e.raw_os_error().unwrap_or(-1)))?;

    Ok(())
}

mod test {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use std::process::{Command, Stdio};

    #[test]
    pub fn test_get_regions_error() {
        let result = get_memory_regions(0, None, None, None);

        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                MemoryError::NoPermission(_) => assert!(true),
                _ => assert!(false),
            }
        } else {
            assert!(false)
        }
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_get_regions_success() {
        let proc = Command::new("./target/debug/examples/simple_program")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match proc {
            Err(e) => assert!(false, "Error running simple program: {e}"),
            Ok(child) => {
                let proc = crate::core::utils::ChildGuard(child);
                let regions = get_memory_regions(proc.0.id(), None, None, None);
                assert!(regions.is_ok());
                let regions = regions.unwrap();
                assert_ne!(regions.len(), 0);
            }
        }
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_get_regions_readonly_success() {
        let proc = Command::new("./target/debug/examples/simple_program")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match proc {
            Err(e) => assert!(false, "Error running simple program: {e}"),
            Ok(child) => {
                let proc = crate::core::utils::ChildGuard(child);
                let regions =
                    get_memory_regions(proc.0.id(), None, None, Some(&[MemoryRegionPerms::Read]));
                assert!(regions.is_ok());
                let regions = regions.unwrap();
                assert_ne!(regions.len(), 0);
            }
        }
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_get_regions_with_range_success() {
        use std::process::{Command, Stdio};

        let proc = Command::new("./target/debug/examples/simple_program")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match proc {
            Err(e) => assert!(false, "Error running simple program: {e}"),
            Ok(child) => {
                let proc = crate::core::utils::ChildGuard(child);
                let regions = get_memory_regions(proc.0.id(), Some(u64::MAX), None, None);
                assert!(regions.is_ok());
                let regions = regions.unwrap();
                assert_eq!(regions.len(), 0);

                let regions = get_memory_regions(proc.0.id(), None, Some(0), None);
                assert!(regions.is_ok());
                let regions = regions.unwrap();
                assert_eq!(regions.len(), 0);
            }
        }
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_read_memory_address_success() {
        use std::io::{BufRead, BufReader};
        use std::process::{Command, Stdio};

        let proc = Command::new("./target/debug/examples/simple_program")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let mut proc = crate::core::utils::ChildGuard(proc);
        let stdout = proc.0.stdout.take().expect("child had no stdout");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();

        reader.read_line(&mut line).unwrap();

        let hex_str = line.trim();
        let address = usize::from_str_radix(hex_str.trim_start_matches("0x"), 16)
            .expect("failed to parse hex");

        let value = read_memory_address(proc.0.id(), address, 4).unwrap();
        let value = u32::from_le_bytes(value.try_into().unwrap());
        assert_eq!(value, 31337_u32);
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_write_memory_address_success() {
        use std::io::{BufRead, BufReader, Write};
        use std::process::{Command, Stdio};

        let proc = Command::new("./target/debug/examples/simple_program")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let mut proc = crate::core::utils::ChildGuard(proc);
        let mut stdin = proc.0.stdin.take().expect("child has no stdin");
        let stdout = proc.0.stdout.take().expect("child had no stdout");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();

        reader.read_line(&mut line).unwrap();

        let hex_str = line.trim();
        let address = usize::from_str_radix(hex_str.trim_start_matches("0x"), 16)
            .expect("failed to parse hex");

        let value = read_memory_address(proc.0.id(), address, 4).unwrap();
        let value = u32::from_le_bytes(value.try_into().unwrap());
        assert_eq!(value, 31337_u32);

        write_memory_address(proc.0.id(), address, &99999_u32.to_le_bytes().to_vec()).unwrap();
        let value = read_memory_address(proc.0.id(), address, 4).unwrap();
        let value = u32::from_le_bytes(value.try_into().unwrap());

        writeln!(stdin, "read").unwrap();
        stdin.flush().unwrap();

        let mut response = String::new();
        reader.read_line(&mut response).unwrap();
        let response_value: u32 = response
            .trim()
            .parse()
            .expect("failed to parse child response as u32");

        assert_eq!(value, 99999);
        assert_eq!(value, response_value);
    }
}
