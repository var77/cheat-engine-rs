use memchr::memmem;

use crate::core::mem::{
    MemoryError, MemoryRegion, get_memory_regions, read_memory_address, write_memory_address,
};

#[derive(Debug, Clone, Copy)]
pub enum ValueType {
    U64,
    I64,
    U32,
    I32,
    String,
}

impl ValueType {
    pub fn get_size(&self) -> u64 {
        match self {
            ValueType::U64 | ValueType::I64 => 8,
            ValueType::U32 | ValueType::I32 => 4,
            ValueType::String => 0,
        }
    }

    pub fn get_string(&self) -> String {
        match self {
            ValueType::U64 => format!("u64 ({}B)", self.get_size()),
            ValueType::I64 => format!("i64 ({}B)", self.get_size()),
            ValueType::U32 => format!("u32 ({}B)", self.get_size()),
            ValueType::I32 => format!("i32 ({}B)", self.get_size()),
            ValueType::String => String::from("string"),
        }
    }

    pub fn get_value_string(&self, value: &[u8]) -> String {
        if value.is_empty() {
            return String::new();
        }

        match self {
            ValueType::U64 => format!("{}", u64::from_le_bytes(value.try_into().unwrap())),
            ValueType::I64 => format!("{}", i64::from_le_bytes(value.try_into().unwrap())),
            ValueType::U32 => format!("{}", u32::from_le_bytes(value.try_into().unwrap())),
            ValueType::I32 => format!("{}", i32::from_le_bytes(value.try_into().unwrap())),
            ValueType::String => String::from_utf8(value.to_vec()).unwrap(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub address: u64,
    pub value_type: ValueType,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum ScanError {
    InvalidValue,
    EmptyValue,
    InvalidAddress,
    AddressMismatch,
    Memory(MemoryError),
}
impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidValue => write!(f, "Invalid scan value provided"),
            Self::EmptyValue => write!(f, "Value is reqeuired to be set before scan"),
            Self::InvalidAddress => write!(f, "Invalid address hex"),
            Self::AddressMismatch => write!(f, "Start address should be smaller than end address"),
            Self::Memory(e) => write!(f, "{e}"),
        }
    }
}

impl ScanResult {
    pub fn new(address: u64, value_type: ValueType, value: Vec<u8>) -> Self {
        ScanResult {
            address,
            value_type,
            value,
        }
    }

    pub fn get_string(&self) -> String {
        self.value_type.get_value_string(self.value.as_slice())
    }
}

#[derive(Debug)]
pub struct Scan {
    pub pid: u32,
    pub value: Vec<u8>,
    pub value_type: ValueType,
    pub results: Vec<ScanResult>,
    pub watchlist: Vec<ScanResult>,
    start_address: Option<u64>,
    end_address: Option<u64>,
    memory_regions: Vec<MemoryRegion>,
}

impl Scan {
    pub fn new(
        pid: u32,
        value: Vec<u8>,
        value_type: ValueType,
        start_address: Option<u64>,
        end_address: Option<u64>,
    ) -> Result<Self, ScanError> {
        let memory_regions =
            get_memory_regions(pid, start_address, end_address).map_err(ScanError::Memory)?;

        Ok(Scan {
            pid,
            value,
            start_address,
            end_address,
            memory_regions,
            value_type,
            results: vec![],
            watchlist: vec![],
        })
    }

    pub fn set_value_type(&mut self, value_type: ValueType) -> Result<(), ScanError> {
        let current_value_string = self.value_type.get_value_string(self.value.as_slice());
        self.value_type = value_type;
        if !self.value.is_empty() {
            self.set_value_from_str(&current_value_string)?;
        }
        Ok(())
    }

    pub fn value_from_str(&mut self, value_str: &str) -> Result<Vec<u8>, ScanError> {
        Ok(match self.value_type {
            ValueType::U64 => value_str
                .parse::<u64>()
                .map_err(|_| ScanError::InvalidValue)?
                .to_le_bytes()
                .to_vec(),
            ValueType::I64 => value_str
                .parse::<i64>()
                .map_err(|_| ScanError::InvalidValue)?
                .to_le_bytes()
                .to_vec(),
            ValueType::U32 => value_str
                .parse::<u32>()
                .map_err(|_| ScanError::InvalidValue)?
                .to_le_bytes()
                .to_vec(),
            ValueType::I32 => value_str
                .parse::<i32>()
                .map_err(|_| ScanError::InvalidValue)?
                .to_le_bytes()
                .to_vec(),
            ValueType::String => value_str.as_bytes().to_vec(),
        })
    }

    pub fn set_value_from_str(&mut self, value_str: &str) -> Result<(), ScanError> {
        self.value = self.value_from_str(value_str)?;

        Ok(())
    }

    fn parse_address_hex(addr_hex: &str) -> Result<Option<u64>, ScanError> {
        if addr_hex.is_empty() {
            Ok(None)
        } else {
            let parsed_addr = u64::from_str_radix(addr_hex.trim_start_matches("0x"), 16)
                .map_err(|_| ScanError::InvalidAddress)?;
            Ok(Some(parsed_addr))
        }
    }

    fn update_memory_regions(&mut self) -> Result<(), ScanError> {
        self.memory_regions = get_memory_regions(self.pid, self.start_address, self.end_address)
            .map_err(ScanError::Memory)?;
        Ok(())
    }

    pub fn set_start_address(&mut self, addr_hex: &str) -> Result<(), ScanError> {
        let parsed_addr = Self::parse_address_hex(addr_hex)?;

        if let (Some(start), Some(end)) = (parsed_addr, self.end_address)
            && start > end
        {
            return Err(ScanError::AddressMismatch);
        }

        self.start_address = parsed_addr;
        self.update_memory_regions()?;

        Ok(())
    }

    pub fn set_end_address(&mut self, addr_hex: &str) -> Result<(), ScanError> {
        let parsed_addr = Self::parse_address_hex(addr_hex)?;

        if let (Some(start), Some(end)) = (self.start_address, parsed_addr)
            && end < start
        {
            return Err(ScanError::AddressMismatch);
        }

        self.end_address = parsed_addr;
        self.update_memory_regions()?;

        Ok(())
    }

    fn scan_region(&self, region: &MemoryRegion) -> Result<Vec<ScanResult>, MemoryError> {
        let mut results: Vec<ScanResult> = Vec::new();
        let mut current_address = region.start as usize;
        let end = region.end as usize;

        let size = self.value.len();
        const BLOCK_SIZE: usize = 0x10000;

        while current_address < end {
            let to_read = std::cmp::min(BLOCK_SIZE, end - current_address);
            if to_read < size {
                break;
            }

            match read_memory_address(self.pid, current_address, to_read) {
                Err(e) => {
                    if let MemoryError::ProcessAttach(_) = e {
                        return Err(e);
                    }
                }
                Ok(val) => {
                    results.extend(memmem::find_iter(&val, &self.value).map(|i| {
                        ScanResult::new(
                            (current_address + i) as u64,
                            self.value_type,
                            self.value.clone(),
                        )
                    }));
                }
            }

            current_address += to_read - (size - 1);
        }

        Ok(results)
    }

    fn check_value(&self) -> Result<(), ScanError> {
        if self.value.is_empty() {
            return Err(ScanError::EmptyValue);
        }

        Ok(())
    }

    fn refresh_watchlist(&mut self) -> Result<(), ScanError> {
        for result in &mut self.watchlist {
            match read_memory_address(self.pid, result.address as usize, result.value.len()) {
                Err(e) => {
                    if let MemoryError::ProcessAttach(_) = e {
                        return Err(ScanError::Memory(e));
                    }
                }
                Ok(val) => result.value = val,
            }
        }

        Ok(())
    }

    pub fn init(&mut self) -> Result<&Vec<ScanResult>, ScanError> {
        self.check_value()?;
        let mut results: Vec<ScanResult> = Vec::new();

        for region in &self.memory_regions {
            results.extend(self.scan_region(region).map_err(ScanError::Memory)?);
        }

        self.results = results;
        self.refresh_watchlist()?;

        Ok(&self.results)
    }

    pub fn refresh(&mut self) -> Result<&Vec<ScanResult>, ScanError> {
        for result in &mut self.results {
            match read_memory_address(self.pid, result.address as usize, result.value.len()) {
                Err(e) => {
                    if let MemoryError::ProcessAttach(_) = e {
                        return Err(ScanError::Memory(e));
                    }
                }
                Ok(val) => result.value = val,
            }
        }

        self.refresh_watchlist()?;

        Ok(&self.results)
    }

    pub fn next_scan(&mut self) -> Result<&Vec<ScanResult>, ScanError> {
        let mut new_results = Vec::with_capacity(self.results.len());
        for result in &mut self.results {
            match read_memory_address(self.pid, result.address as usize, result.value.len()) {
                Err(e) => {
                    if let MemoryError::ProcessAttach(_) = e {
                        return Err(ScanError::Memory(e));
                    }
                }
                Ok(val) => {
                    if val == self.value {
                        let mut new_result = result.clone();
                        new_result.value = val;
                        new_results.push(new_result);
                    }
                }
            }
        }

        self.results = new_results;
        self.refresh_watchlist()?;

        Ok(&self.results)
    }

    pub fn add_to_watchlist(&mut self, result: ScanResult) {
        let already_existing = self
            .watchlist
            .iter()
            .position(|w| w.address == result.address);
        if already_existing.is_some() {
            return;
        }

        self.watchlist.push(result);
    }

    pub fn remove_from_watchlist(&mut self, address: u64) {
        let already_existing = self.watchlist.iter().position(|w| w.address == address);
        if already_existing.is_none() {
            return;
        }

        self.watchlist.remove(already_existing.unwrap());
    }

    pub fn update_value(&mut self, address: u64, value_str: &str) -> Result<(), ScanError> {
        let value = self.value_from_str(value_str)?;
        write_memory_address(self.pid, address as usize, &value).map_err(ScanError::Memory)?;
        Ok(())
    }
}

mod test {
    #[allow(unused_imports)]
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

        scan.update_value(address as u64, "333333").unwrap();

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

    #[test]
    pub fn test_set_value_from_str_u64_success() {
        use super::*;
        let mut scan = Scan {
            pid: 0,
            value: vec![],
            value_type: ValueType::U64,
            results: vec![],
            watchlist: vec![],
            start_address: None,
            end_address: None,
            memory_regions: vec![],
        };

        let result = scan.set_value_from_str("12345");
        assert!(result.is_ok());
        assert_eq!(scan.value, 12345_u64.to_le_bytes().to_vec());
    }

    #[test]
    pub fn test_set_value_from_str_i64_success() {
        use super::*;
        let mut scan = Scan {
            pid: 0,
            value: vec![],
            value_type: ValueType::I64,
            results: vec![],
            watchlist: vec![],
            start_address: None,
            end_address: None,
            memory_regions: vec![],
        };

        let result = scan.set_value_from_str("-54321");
        assert!(result.is_ok());
        assert_eq!(scan.value, (-54321_i64).to_le_bytes().to_vec());
    }

    #[test]
    pub fn test_set_value_from_str_u32_success() {
        use super::*;
        let mut scan = Scan {
            pid: 0,
            value: vec![],
            value_type: ValueType::U32,
            results: vec![],
            watchlist: vec![],
            start_address: None,
            end_address: None,
            memory_regions: vec![],
        };

        let result = scan.set_value_from_str("31337");
        assert!(result.is_ok());
        assert_eq!(scan.value, 31337_u32.to_le_bytes().to_vec());
    }

    #[test]
    pub fn test_set_value_from_str_i32_success() {
        use super::*;
        let mut scan = Scan {
            pid: 0,
            value: vec![],
            value_type: ValueType::I32,
            results: vec![],
            watchlist: vec![],
            start_address: None,
            end_address: None,
            memory_regions: vec![],
        };

        let result = scan.set_value_from_str("-999");
        assert!(result.is_ok());
        assert_eq!(scan.value, (-999_i32).to_le_bytes().to_vec());
    }

    #[test]
    pub fn test_set_value_from_str_invalid_value() {
        use super::*;
        let mut scan = Scan {
            pid: 0,
            value: vec![],
            value_type: ValueType::U32,
            results: vec![],
            watchlist: vec![],
            start_address: None,
            end_address: None,
            memory_regions: vec![],
        };

        let result = scan.set_value_from_str("not_a_number");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ScanError::InvalidValue));
    }

    #[test]
    pub fn test_set_value_from_str_overflow() {
        use super::*;
        let mut scan = Scan {
            pid: 0,
            value: vec![],
            value_type: ValueType::U32,
            results: vec![],
            watchlist: vec![],
            start_address: None,
            end_address: None,
            memory_regions: vec![],
        };

        // This value is too large for u32
        let result = scan.set_value_from_str("99999999999999");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ScanError::InvalidValue));
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_set_start_address_success() {
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

                let mut scan = Scan::new(
                    proc.0.id(),
                    31337_u32.to_le_bytes().to_vec(),
                    ValueType::U32,
                    None,
                    None,
                )
                .unwrap();

                let result = scan.set_start_address("0x1000");
                assert!(result.is_ok());
                assert_eq!(scan.start_address, Some(0x1000));
            }
        }
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_set_start_address_without_prefix() {
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

                let mut scan = Scan::new(
                    proc.0.id(),
                    31337_u32.to_le_bytes().to_vec(),
                    ValueType::U32,
                    None,
                    None,
                )
                .unwrap();

                let result = scan.set_start_address("ABCD");
                assert!(result.is_ok());
                assert_eq!(scan.start_address, Some(0xABCD));
            }
        }
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_set_start_address_empty_clears() {
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

                let mut scan = Scan::new(
                    proc.0.id(),
                    31337_u32.to_le_bytes().to_vec(),
                    ValueType::U32,
                    Some(0x1000),
                    None,
                )
                .unwrap();

                assert_eq!(scan.start_address, Some(0x1000));
                let result = scan.set_start_address("");
                assert!(result.is_ok());
                assert_eq!(scan.start_address, None);
            }
        }
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_set_start_address_invalid_hex() {
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

                let mut scan = Scan::new(
                    proc.0.id(),
                    31337_u32.to_le_bytes().to_vec(),
                    ValueType::U32,
                    None,
                    None,
                )
                .unwrap();

                let result = scan.set_start_address("0xGHIJ");
                assert!(result.is_err());
                assert!(matches!(result.unwrap_err(), ScanError::InvalidAddress));
            }
        }
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_set_start_address_mismatch_with_end() {
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

                let mut scan = Scan::new(
                    proc.0.id(),
                    31337_u32.to_le_bytes().to_vec(),
                    ValueType::U32,
                    None,
                    Some(0x1000),
                )
                .unwrap();

                // Try to set start address greater than end address
                let result = scan.set_start_address("0x2000");
                assert!(result.is_err());
                assert!(matches!(result.unwrap_err(), ScanError::AddressMismatch));
            }
        }
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_set_end_address_success() {
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

                let mut scan = Scan::new(
                    proc.0.id(),
                    31337_u32.to_le_bytes().to_vec(),
                    ValueType::U32,
                    None,
                    None,
                )
                .unwrap();

                let result = scan.set_end_address("0xFFFFFFFF");
                assert!(result.is_ok());
                assert_eq!(scan.end_address, Some(0xFFFFFFFF));
            }
        }
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_set_end_address_without_prefix() {
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

                let mut scan = Scan::new(
                    proc.0.id(),
                    31337_u32.to_le_bytes().to_vec(),
                    ValueType::U32,
                    None,
                    None,
                )
                .unwrap();

                let result = scan.set_end_address("DEED");
                assert!(result.is_ok());
                assert_eq!(scan.end_address, Some(0xDEED));
            }
        }
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_set_end_address_empty_clears() {
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

                let mut scan = Scan::new(
                    proc.0.id(),
                    31337_u32.to_le_bytes().to_vec(),
                    ValueType::U32,
                    None,
                    Some(0xFFFF),
                )
                .unwrap();

                assert_eq!(scan.end_address, Some(0xFFFF));
                let result = scan.set_end_address("");
                assert!(result.is_ok());
                assert_eq!(scan.end_address, None);
            }
        }
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_set_end_address_invalid_hex() {
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

                let mut scan = Scan::new(
                    proc.0.id(),
                    31337_u32.to_le_bytes().to_vec(),
                    ValueType::U32,
                    None,
                    None,
                )
                .unwrap();

                let result = scan.set_end_address("0xXYZ");
                assert!(result.is_err());
                assert!(matches!(result.unwrap_err(), ScanError::InvalidAddress));
            }
        }
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_set_end_address_mismatch_with_start() {
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

                let mut scan = Scan::new(
                    proc.0.id(),
                    31337_u32.to_le_bytes().to_vec(),
                    ValueType::U32,
                    Some(0x2000),
                    None,
                )
                .unwrap();

                // Try to set end address smaller than start address
                let result = scan.set_end_address("0x1000");
                assert!(result.is_err());
                assert!(matches!(result.unwrap_err(), ScanError::AddressMismatch));
            }
        }
    }

    #[test]
    pub fn test_add_to_watchlist_success() {
        use super::*;
        let mut scan = Scan {
            pid: 0,
            value: vec![],
            value_type: ValueType::U32,
            results: vec![],
            watchlist: vec![],
            start_address: None,
            end_address: None,
            memory_regions: vec![],
        };

        let result1 = ScanResult::new(0x1000, ValueType::U32, vec![1, 2, 3, 4]);
        let result2 = ScanResult::new(0x2000, ValueType::U32, vec![5, 6, 7, 8]);

        scan.add_to_watchlist(result1);
        assert_eq!(scan.watchlist.len(), 1);
        assert_eq!(scan.watchlist[0].address, 0x1000);

        scan.add_to_watchlist(result2);
        assert_eq!(scan.watchlist.len(), 2);
        assert_eq!(scan.watchlist[1].address, 0x2000);
    }

    #[test]
    pub fn test_add_to_watchlist_duplicate_ignores() {
        use super::*;
        let mut scan = Scan {
            pid: 0,
            value: vec![],
            value_type: ValueType::U32,
            results: vec![],
            watchlist: vec![],
            start_address: None,
            end_address: None,
            memory_regions: vec![],
        };

        let result = ScanResult::new(0x1000, ValueType::U32, vec![1, 2, 3, 4]);

        scan.add_to_watchlist(result.clone());
        assert_eq!(scan.watchlist.len(), 1);

        // Try to add the same address again
        scan.add_to_watchlist(result);
        assert_eq!(scan.watchlist.len(), 1);
    }

    #[test]
    pub fn test_remove_from_watchlist_success() {
        use super::*;
        let mut scan = Scan {
            pid: 0,
            value: vec![],
            value_type: ValueType::U32,
            results: vec![],
            watchlist: vec![],
            start_address: None,
            end_address: None,
            memory_regions: vec![],
        };

        let result1 = ScanResult::new(0x1000, ValueType::U32, vec![1, 2, 3, 4]);
        let result2 = ScanResult::new(0x2000, ValueType::U32, vec![5, 6, 7, 8]);

        scan.add_to_watchlist(result1.clone());
        scan.add_to_watchlist(result2.clone());
        assert_eq!(scan.watchlist.len(), 2);

        scan.remove_from_watchlist(result1.address);
        assert_eq!(scan.watchlist.len(), 1);
        assert_eq!(scan.watchlist[0].address, 0x2000);
    }

    #[test]
    pub fn test_remove_from_watchlist_not_present() {
        use super::*;
        let mut scan = Scan {
            pid: 0,
            value: vec![],
            value_type: ValueType::U32,
            results: vec![],
            watchlist: vec![],
            start_address: None,
            end_address: None,
            memory_regions: vec![],
        };

        let result1 = ScanResult::new(0x1000, ValueType::U32, vec![1, 2, 3, 4]);
        let result2 = ScanResult::new(0x2000, ValueType::U32, vec![5, 6, 7, 8]);

        scan.add_to_watchlist(result1);
        assert_eq!(scan.watchlist.len(), 1);

        // Try to remove an address that's not in the watchlist
        scan.remove_from_watchlist(result2.address);
        assert_eq!(scan.watchlist.len(), 1);
        assert_eq!(scan.watchlist[0].address, 0x1000);
    }

    #[test]
    pub fn test_remove_from_watchlist_empty() {
        use super::*;
        let mut scan = Scan {
            pid: 0,
            value: vec![],
            value_type: ValueType::U32,
            results: vec![],
            watchlist: vec![],
            start_address: None,
            end_address: None,
            memory_regions: vec![],
        };

        let result = ScanResult::new(0x1000, ValueType::U32, vec![1, 2, 3, 4]);

        // Try to remove from empty watchlist
        scan.remove_from_watchlist(result.address);
        assert_eq!(scan.watchlist.len(), 0);
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_refresh_watchlist_success() {
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

        let mut scan = Scan::new(
            proc.0.id(),
            31337_u32.to_le_bytes().to_vec(),
            ValueType::U32,
            None,
            None,
        )
        .unwrap();

        // Initialize scan to get results
        let results = scan.init().unwrap();
        assert_eq!(results.len(), 1);

        // Add result to watchlist
        scan.add_to_watchlist(scan.results[0].clone());
        assert_eq!(scan.watchlist.len(), 1);
        assert_eq!(
            u32::from_le_bytes(scan.watchlist[0].value.as_slice().try_into().unwrap()),
            31337_u32
        );

        // Modify the memory value
        write_memory_address(proc.0.id(), address, &999999_u32.to_le_bytes().to_vec()).unwrap();

        // Refresh the watchlist
        scan.refresh_watchlist().unwrap();

        // Check that watchlist value was updated
        assert_eq!(scan.watchlist.len(), 1);
        assert_eq!(
            u32::from_le_bytes(scan.watchlist[0].value.as_slice().try_into().unwrap()),
            999999_u32
        );
    }

    #[test]
    #[ignore = "requires root"]
    pub fn test_refresh_watchlist_multiple_entries() {
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

        let mut scan = Scan::new(
            proc.0.id(),
            31337_u32.to_le_bytes().to_vec(),
            ValueType::U32,
            None,
            None,
        )
        .unwrap();

        // Initialize scan
        scan.init().unwrap();
        assert_eq!(scan.results.len(), 1);

        // Add to watchlist
        scan.add_to_watchlist(scan.results[0].clone());

        // Add a fake entry to watchlist to test multiple entries
        let fake_result =
            ScanResult::new(address as u64 + 100, ValueType::U32, vec![10, 20, 30, 40]);
        scan.add_to_watchlist(fake_result);
        assert_eq!(scan.watchlist.len(), 2);

        // Modify the memory value
        write_memory_address(proc.0.id(), address, &888888_u32.to_le_bytes().to_vec()).unwrap();

        // Refresh the watchlist
        scan.refresh_watchlist().unwrap();

        // Check that first watchlist entry was updated
        assert_eq!(scan.watchlist.len(), 2);
        assert_eq!(
            u32::from_le_bytes(scan.watchlist[0].value.as_slice().try_into().unwrap()),
            888888_u32
        );
    }
}
