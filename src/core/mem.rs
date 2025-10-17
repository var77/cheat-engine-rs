use process_memory::*;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum MemoryError {
    NoPermission(i32),
    MemRead(i32),
    ProcessAttach(i32),
}

impl Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPermission(code) => write!(f, "Permission Denied: OS Error ({code})"),
            Self::MemRead(code) => write!(f, "Could not read memory: OS Error ({code})"),
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
    pub perms: String,
}

#[cfg(target_os = "macos")]
pub fn get_memory_regions(
    pid: u32,
    start: Option<u64>,
    end: Option<u64>,
) -> Result<Vec<MemoryRegion>, MemoryError> {
    use mach_sys::{
        kern_return::{KERN_INVALID_ADDRESS, KERN_SUCCESS},
        port::mach_port_name_t,
        traps::{mach_task_self, task_for_pid},
        vm::mach_vm_region,
        vm_prot::{VM_PROT_EXECUTE, VM_PROT_READ, VM_PROT_WRITE},
        vm_region::{VM_REGION_BASIC_INFO_64, vm_region_info_t},
        vm_types::{mach_vm_address_t, mach_vm_size_t, vm_map_t},
    };
    use mach_sys::{port::mach_port_t, vm_region::vm_region_basic_info_data_64_t};

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

        if info.protection & VM_PROT_WRITE == 0 {
            // skip non-writable regions
            address += size;
            continue;
        }

        let mut perms = String::new();
        if info.protection & VM_PROT_READ != 0 {
            perms.push('r');
        } else {
            perms.push('-');
        }
        if info.protection & VM_PROT_WRITE != 0 {
            perms.push('w');
        } else {
            perms.push('-');
        }
        if info.protection & VM_PROT_EXECUTE != 0 {
            perms.push('x');
        } else {
            perms.push('-');
        }

        regions.push(MemoryRegion {
            start: address,
            end: address + size,
            perms,
        });

        address += size;
    }

    Ok(regions)
}

pub fn read_memory_address(pid: u32, addr: usize, size: usize) -> Result<Vec<u8>, MemoryError> {
    let handle = (pid as Pid).try_into_process_handle();

    if let Err(e) = handle {
        return Err(MemoryError::ProcessAttach(e.raw_os_error().unwrap_or(-1)));
    }

    let handle = handle.unwrap();

    let mut result = vec![0; size];
    if let Err(e) = handle.copy_address(addr, &mut result) {
        return Err(MemoryError::MemRead(e.raw_os_error().unwrap_or(-1)));
    }

    Ok(result)
}

pub fn write_memory_address(pid: u32, addr: usize, value: &[u8]) -> Result<(), MemoryError> {
    let handle = (pid as Pid).try_into_process_handle();

    if let Err(e) = handle {
        return Err(MemoryError::ProcessAttach(e.raw_os_error().unwrap_or(-1)));
    }

    let handle = handle.unwrap();

    if let Err(e) = handle.put_address(addr, value) {
        return Err(MemoryError::MemRead(e.raw_os_error().unwrap_or(-1)));
    }

    Ok(())
}

mod test {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use std::process::{Command, Stdio};

    #[test]
    pub fn test_get_regions_error() {
        let result = get_memory_regions(0, None, None);

        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                MemoryError::NoPermission(code) => assert_eq!(code, 5),
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
                let regions = get_memory_regions(proc.0.id(), None, None);
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
                let regions = get_memory_regions(proc.0.id(), Some(u64::MAX), None);
                assert!(regions.is_ok());
                let regions = regions.unwrap();
                assert_eq!(regions.len(), 0);

                let regions = get_memory_regions(proc.0.id(), None, Some(0));
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
