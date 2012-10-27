use std::map::{HashMap};
use Path = path::Path;
use std::getopts::*;
use std::time::*;

pub fn title_case(s: &str) -> ~str
{
	if s.is_not_empty() && char::is_lowercase(s[0] as char)
	{
		s.substr(0, 1).to_upper() + s.slice(1, s.len())
	}
	else
	{
		s.to_unique()
	}
}

pub fn remove_entry_if<K: cmp::Eq to_bytes::IterBytes hash::Hash Copy, V: Copy>(map: HashMap<K, V>, predicate: fn (key: K, value: V) -> bool)
{
	let mut remove = ~[];
	
	for map.each |key, value|
	{
		if predicate(key, value)
		{
			vec::push(&mut remove, key);
		}
	}
	
	for remove.each |key|
	{
		map.remove(*key);
	}
}

/// Returns an error if the files cannot be copied.
pub fn scp_files(files: &[~Path], user: &str, host: &str) -> option::Option<~str>
{
	if vec::is_empty(files)
	{
		return option::Some(~"No files were found to copy");
	}
	
	let args = do files.map |f| {(**f).to_str()} + ~[fmt!("%s@%s:", user, host)];	// need to de-reference f or we get a ~ in front of the string
	
	info!("scp %s", str::connect(args, ~" "));
	run_command(~"scp", args)
}

/// Uses ssh to run a command remotely.
///
/// Returns an error if the command returned a non-zero result code
pub fn run_remote_command(user: &str, host: &str, command: &str) -> option::Option<~str>
{
	let args = ~[fmt!("%s@%s", user, host)] + ~[command.to_unique()];
	
	info!("ssh %s \"%s\"", args.head(), str::connect(args.tail(), ~" "));
	run_command(~"ssh", args)
}

/// Returns paths to files in dir with an extension in extensions.
///
/// Returned paths include the dir component.
pub fn list_dir_path(dir: &Path, extensions: &[~str]) -> ~[~Path]
{
	let files = core::os::list_dir_path(dir);
	do files.filter |file|
	{
		let ftype = file.filetype();
		assert ftype.is_none() || ftype.get().starts_with(".");
		match ftype
		{
			option::Some(ref ext) 	=> extensions.contains(ext),
			option::None		=> false,
		}
	}
}

// Like time::precise_time_s except that a lower resolution (and presumbably much faster)
// timer is used.
pub fn imprecise_time_s() -> float
{
	let time = std::time::get_time();
	let secs = time.sec as float;
	let secs = secs + (time.nsec as float)/1000_000_000.0;
	return secs;
}

// Takes a tm and returns the number of seconds from the current
// time and strings like "2 minutes ago", "Yesterday 18:06", and
// "Thu Jan  1 00:00:00 1970".
pub fn tm_to_delta_str(time: Tm) -> {elapsed: float, delta: ~str}
{
	fn tm_to_secs(time: Tm) -> float
	{
		let {sec: seconds, nsec: nanosecs} = time.to_timespec();
		seconds as float + (nanosecs as float)*0.000_000_001
	}
	
	fn tm_to_delta_str_same_day(elapsed: float) -> ~str
	{
		assert elapsed >= 0.0;
		assert elapsed < 24.0*60.0*60.0;
		
		let (value, units) =
			if elapsed < 60.0
			{
				(elapsed, ~"second")
			}
			else if elapsed < 60.0*60.0
			{
				(elapsed/60.0, ~"minute")
			}
			else
			{
				(elapsed/(60.0*60.0), ~"hour")
			};
			
		let value = float::round(value as f64) as int;
		let units = if value == 1 {units} else {units + ~"s"};
		fmt!("%? %s", value, units)
	}
	
	let current = now();
	let time_secs = tm_to_secs(copy time);
	let current_secs = tm_to_secs(copy current);
	let elapsed = float::abs(time_secs - current_secs);
	
	if time_secs == current_secs
	{
		// "now"
		{elapsed: elapsed, delta: ~"now"}
	}
	else if time.tm_yday == current.tm_yday
	{
		// "2 minutes ago", "4 hours from now"
		let suffix = if time_secs < current_secs {~" ago"} else {~" from now"};
		{elapsed: elapsed, delta: tm_to_delta_str_same_day(elapsed) + suffix}
	}
	else if i32::abs(time.tm_yday - current.tm_yday) == 1
	{
		// "Yesterday 18:06"
		let prefix = if time_secs < current_secs {~"Yesterday "} else {~"Tomorrow "};
		{elapsed: elapsed, delta: prefix + fmt!("%02d:%02d", time.tm_hour as int, time.tm_min as int)}
	}
	else
	{
		// "Thu Jan  1 00:00:00 1970"
		{elapsed: elapsed, delta: time.ctime()}
	}
}

// --------------------------------------------------------------------------------------
priv fn run_command(tool: &str, args: &[~str]) -> option::Option<~str>
{
	match core::run::program_output(tool, args)
	{
		{status: 0, _} =>
		{
			option::None
		}
		{status: code, out: _, err: ~""} =>
		{
			option::Some(fmt!("%s result code was %?", tool, code))
		}
		{status: code, out: _, err: ref err} =>
		{
			option::Some(fmt!("%s result code was %? (%s)", tool, code, *err))
		}
	}
}
