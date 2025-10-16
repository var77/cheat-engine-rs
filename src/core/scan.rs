use crate::core::mem::{MemoryError, MemoryRegion, get_memory_regions, read_memory_address};

#[derive(Debug, Clone)]
pub enum ValueType {
    U64,
    I64,
    U32,
    I32,
}

impl ValueType {
    pub fn get_size(&self) -> u64 {
        match self {
            ValueType::U64 | ValueType::I64 => 8,
            ValueType::U32 | ValueType::I32 => 4,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            ValueType::U64 => format!("u64 ({}B)", self.get_size()),
            ValueType::I64 => format!("i64 ({}B)", self.get_size()),
            ValueType::U32 => format!("u32 ({}B)", self.get_size()),
            ValueType::I32 => format!("i32 ({}B)", self.get_size()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub address: u64,
    pub value_type: ValueType,
    pub value: Vec<u8>,
}

impl ScanResult {
    pub fn new(address: u64, value_type: ValueType, value: Vec<u8>) -> Self {
        ScanResult {
            address,
            value_type,
            value,
        }
    }

    pub fn to_string(&self) -> String {
        match self.value_type {
            ValueType::U64 => format!(
                "{}",
                u64::from_le_bytes(self.value.as_slice().try_into().unwrap())
            ),
            ValueType::I64 => format!(
                "{}",
                i64::from_le_bytes(self.value.as_slice().try_into().unwrap())
            ),
            ValueType::U32 => format!(
                "{}",
                u32::from_le_bytes(self.value.as_slice().try_into().unwrap())
            ),
            ValueType::I32 => format!(
                "{}",
                i32::from_le_bytes(self.value.as_slice().try_into().unwrap())
            ),
        }
    }
}

#[derive(Debug)]
pub struct Scan {
    pub pid: u32,
    pub value: Vec<u8>,
    pub value_type: ValueType,
    pub results: Vec<ScanResult>,
    memory_regions: Vec<MemoryRegion>,
}

impl Scan {
    pub fn new(
        pid: u32,
        value: Vec<u8>,
        value_type: ValueType,
        start_address: Option<u64>,
        end_address: Option<u64>,
    ) -> Result<Self, MemoryError> {
        let memory_regions = get_memory_regions(pid, start_address, end_address)?;

        Ok(Scan {
            pid,
            value,
            memory_regions,
            value_type,
            results: vec![],
        })
    }

    fn scan_region(&self, region: &MemoryRegion) -> Result<Vec<ScanResult>, MemoryError> {
        let mut results: Vec<ScanResult> = Vec::new();
        let mut current_address = region.start as usize;
        let end = region.end as usize;

        let size = self.value_type.get_size() as usize;
        const BLOCK_SIZE: usize = 0x10000;

        while current_address < end {
            let to_read = std::cmp::min(BLOCK_SIZE, end - current_address);
            if to_read < size {
                break;
            }

            match read_memory_address(self.pid, current_address, to_read) {
                Err(e) => match e {
                    MemoryError::ProcessAttachError(_) => return Err(e),
                    _ => {}
                },
                Ok(val) => {
                    for i in 0..=to_read.saturating_sub(size) {
                        if i + size < val.len() && self.value == &val[i..i + size] {
                            results.push(ScanResult::new(
                                (current_address + i) as u64,
                                self.value_type.clone(),
                                val[i..i + size].to_vec(),
                            ));
                        }
                    }
                }
            }

            current_address += to_read - (size - 1);
        }

        Ok(results)
    }

    pub fn init(&mut self) -> Result<&Vec<ScanResult>, MemoryError> {
        let mut results: Vec<ScanResult> = Vec::new();

        for region in &self.memory_regions {
            results.extend(self.scan_region(region)?);
        }

        self.results = results;

        Ok(&self.results)
    }

    pub fn refresh(&mut self) -> Result<&Vec<ScanResult>, MemoryError> {
        for result in &mut self.results {
            match read_memory_address(
                self.pid,
                result.address as usize,
                result.value_type.get_size() as usize,
            ) {
                Err(e) => match e {
                    MemoryError::ProcessAttachError(_) => return Err(e),
                    _ => {}
                },
                Ok(val) => result.value = val,
            }
        }

        Ok(&self.results)
    }

    pub fn next_scan(&mut self) -> Result<&Vec<ScanResult>, MemoryError> {
        let mut new_results = Vec::with_capacity(self.results.len());
        for result in &mut self.results {
            match read_memory_address(
                self.pid,
                result.address as usize,
                result.value_type.get_size() as usize,
            ) {
                Err(e) => match e {
                    MemoryError::ProcessAttachError(_) => return Err(e),
                    _ => {}
                },
                Ok(val) => {
                    if val == self.value {
                        new_results.push(result.clone());
                    }
                }
            }
        }

        self.results = new_results;
        Ok(&self.results)
    }
}

mod test {
    use crate::core::mem::write_memory_address;

    #[test]
    #[ignore = "requires root"]
    pub fn test_scan_creation_success() {
        use super::*;
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

                let scan = Scan::new(
                    proc.0.id(),
                    31337_u32.to_le_bytes().to_vec(),
                    ValueType::U32,
                    None,
                    None,
                );
                assert!(scan.is_ok());
                let scan = scan.unwrap();
                assert_eq!(scan.results.len(), 0);
            }
        }
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_scan_init_success() {
        use super::*;
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

        let scan = Scan::new(
            proc.0.id(),
            31337_u32.to_le_bytes().to_vec(),
            ValueType::U32,
            None,
            None,
        );
        assert!(scan.is_ok());
        let mut scan = scan.unwrap();
        let results = scan.init().unwrap();
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.address, address as u64);
        assert_eq!(
            u32::from_le_bytes(result.value.as_slice().try_into().unwrap()),
            31337_u32
        );
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_scan_refresh_success() {
        use super::*;
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

        let scan = Scan::new(
            proc.0.id(),
            31337_u32.to_le_bytes().to_vec(),
            ValueType::U32,
            None,
            None,
        );
        assert!(scan.is_ok());
        let mut scan = scan.unwrap();
        let results = scan.init().unwrap();
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.address, address as u64);
        assert_eq!(
            u32::from_le_bytes(result.value.as_slice().try_into().unwrap()),
            31337_u32
        );

        write_memory_address(proc.0.id(), address, &333333_u32.to_le_bytes().to_vec()).unwrap();

        let results = scan.refresh().unwrap();
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.address, address as u64);
        assert_eq!(
            u32::from_le_bytes(result.value.as_slice().try_into().unwrap()),
            333333_u32
        );
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_next_scan_success() {
        use super::*;
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

        let scan = Scan::new(
            proc.0.id(),
            31337_u32.to_le_bytes().to_vec(),
            ValueType::U32,
            None,
            None,
        );
        assert!(scan.is_ok());
        let mut scan = scan.unwrap();
        let results = scan.init().unwrap();
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.address, address as u64);
        assert_eq!(
            u32::from_le_bytes(result.value.as_slice().try_into().unwrap()),
            31337_u32
        );

        let results = scan.next_scan().unwrap();
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.address, address as u64);
        assert_eq!(
            u32::from_le_bytes(result.value.as_slice().try_into().unwrap()),
            31337_u32
        );

        write_memory_address(proc.0.id(), address, &333333_u32.to_le_bytes().to_vec()).unwrap();

        let results = scan.next_scan().unwrap();
        assert_eq!(results.len(), 0);
    }
}
