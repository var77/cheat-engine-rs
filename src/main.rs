mod mem;
mod proc;
mod scan;
mod utils;

fn main() {
    for process in proc::get_list(Some("simple_program")) {
        println!("Pid: {}, Name: {}", process.pid, process.name);
        let memory_regions = mem::get_memory_regions(process.pid, None, None);
        println!("Regions: {:?}", memory_regions);
        break;
    }
}

/*
*
*
* let mut scan = Scan::new(pid, value, size, region (Region::Writable, Region::Range(start, end)));
* let results = scan.init(); // will perform full scan
* let results = scan.next(); // will iterate over existing scan for the same value
* scan.set_value(new_val)
* let results = scan.next(); // will iterate over existing scan for the same value
*
* results: Vec<(str, val)>
*
* write_mem(pid, addr, bytes)
*
*
*
*
*
* */
