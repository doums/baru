use crate::error::Error;
use crate::{read_and_trim, Config, Refresh};
use regex::Regex;

const PROC_MEMINFO: &'static str = "/proc/meminfo";

#[derive(Debug)]
struct MemRegex {
    total: Regex,
    free: Regex,
    buffers: Regex,
    cached: Regex,
    s_reclaimable: Regex,
}

impl MemRegex {
    fn new() -> Self {
        MemRegex {
            total: Regex::new(r"(?m)^MemTotal:\s*(\d+)\s*kB$").unwrap(),
            free: Regex::new(r"(?m)^MemFree:\s*(\d+)\s*kB$").unwrap(),
            buffers: Regex::new(r"(?m)^Buffers:\s*(\d+)\s*kB$").unwrap(),
            cached: Regex::new(r"(?m)^Cached:\s*(\d+)\s*kB$").unwrap(),
            s_reclaimable: Regex::new(r"(?m)^SReclaimable:\s*(\d+)\s*kB$").unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Memory<'a> {
    proc_meminfo: &'a str,
    config: &'a Config,
    mem_regex: MemRegex,
}

impl<'a> Memory<'a> {
    pub fn with_config(config: &'a Config) -> Self {
        Memory {
            proc_meminfo: match &config.proc_meminfo {
                Some(val) => &val,
                None => PROC_MEMINFO,
            },
            mem_regex: MemRegex::new(),
            config,
        }
    }
}

impl<'a> Refresh for Memory<'a> {
    fn refresh(&mut self) -> Result<String, Error> {
        let meminfo = read_and_trim(self.proc_meminfo)?;
        let total = find_meminfo(
            &self.mem_regex.total,
            &meminfo,
            &format!("MemTotal not found in \"{}\"", PROC_MEMINFO),
        )?;
        let free = find_meminfo(
            &self.mem_regex.free,
            &meminfo,
            &format!("MemFree not found in \"{}\"", PROC_MEMINFO),
        )?;

        let buffers = find_meminfo(
            &self.mem_regex.buffers,
            &meminfo,
            &format!("Buffers not found in \"{}\"", PROC_MEMINFO),
        )?;
        let cached = find_meminfo(
            &self.mem_regex.cached,
            &meminfo,
            &format!("Cached not found in \"{}\"", PROC_MEMINFO),
        )?;
        let s_reclaimable = find_meminfo(
            &self.mem_regex.s_reclaimable,
            &meminfo,
            &format!("SReclaimable not found in \"{}\"", PROC_MEMINFO),
        )?;
        let used = total - free - buffers - cached - s_reclaimable;
        let percentage = (used as f64 * 100_f64 / total as f64).round() as i32;
        let total_go = (1024_f64 * (total as f64)) / 1_000_000_000_f64;
        let used_go = 1024_f64 * (used as f64) / 1_000_000_000_f64;
        let mut color = &self.config.default_color;
        if percentage > 90 {
            color = &self.config.red;
        }
        Ok(format!(
            "{}{:.1}{}/{:.1}Go{}󰍛{}",
            color,
            used_go,
            self.config.default_color,
            total_go,
            &self.config.icon_font,
            &self.config.default_font
        ))
    }
}

fn find_meminfo<'a>(regex: &Regex, meminfo: &'a str, error: &'a str) -> Result<i32, String> {
    let matched = regex
        .captures(&meminfo)
        .ok_or_else(|| error.to_string())?
        .get(1)
        .ok_or_else(|| error.to_string())?
        .as_str();
    Ok(matched
        .parse::<i32>()
        .map_err(|err| format!("error while parsing meminfo: {}", err))?)
}
