import std::getopts::*;
import std::time::*;

export scp_files, run_remote_command, list_dir_path, imprecise_time_s, i64_to_unit_str, f64_to_unit_str,
	tm_to_delta_str, title_case;
	
fn title_case(s: ~str) -> ~str 
{
	if s.is_not_empty() && char::is_lowercase(s[0] as char)
	{
		s.substr(0, 1).to_upper() + s.slice(1, s.len())
	}
	else
	{
		s
	}
}

/// Returns an error if the files cannot be copied.
fn scp_files(files: ~[~str], user: ~str, host: ~str) -> option::option<~str>
{
	if vec::is_empty(files)
	{
		ret option::some(~"No files were found to copy");
	}
	
	let args = files + ~[#fmt["%s@%s:", user, host]];
	
	#info["scp %s", str::connect(args, ~" ")];
	run_command(~"scp", args)
}

/// Uses ssh to run a command remotely.
///
/// Returns an error if the command returned a non-zero result code
fn run_remote_command(user: ~str, host: ~str, command: ~str) -> option::option<~str>
{
	let args = ~[#fmt["%s@%s", user, host]] + ~[command];
	
	#info["ssh %s \"%s\"", args.head(), str::connect(args.tail(), ~" ")];
	run_command(~"ssh", args)
}

/// Returns paths to files in dir with an extension in extensions.
///
/// Returned paths include the dir component.
fn list_dir_path(dir: ~str, extensions: ~[~str]) -> ~[~str]
{
	let files = core::os::list_dir_path(dir);
	do files.filter
	|file|
	{
		let (_path, ext) = core::path::splitext(file);
		extensions.contains(ext)
	}
}

fn opt_str_or_default(match: match, name: ~str, default: ~str) -> ~str
{
	if opt_present(match, name)
	{
		opt_str(match, name)
	}
	else
	{
		default
	}
}

fn opt_strs_or_default(match: match, name: ~str, default: ~[~str]) -> ~[~str]
{
	if opt_present(match, name)
	{
		opt_strs(match, name)
	}
	else
	{
		default
	}
}

// Like time::precise_time_s except that a lower resolution (and presumbably much faster)
// timer is used.
fn imprecise_time_s() -> float
{
	let time = std::time::get_time();
	let secs = time.sec as float;
	let secs = secs + (time.nsec as float)/1000_000_000.0;
	ret secs;
}

// Takes a floating point value and returns a string like "234", "200K", "3.2M", etc.
fn f64_to_unit_str(value: f64) -> ~str
{
	if value < 1.0f64
	{
		#fmt["%.1f", value as float]
	}
	else
	{
		i64_to_unit_str(value as i64)
	}
}

// Takes an integer value and returns a string like "234", "200K", "3.2M", etc.
fn i64_to_unit_str(value: i64) -> ~str
{
	if value < 10*1024
	{
		#fmt["%?", value]
	}
	else if value < 1024*1024
	{
		#fmt["%?K", value/1024]
	}
	else
	{
		#fmt["%.1fM", (value as float)/(1024.0*1024.0)]
	}
}

// Takes a tm and returns the number of seconds from the current
// time and strings like "2 minutes ago", "Yesterday 18:06", and
// "Thu Jan  1 00:00:00 1970".
fn tm_to_delta_str(time: tm) -> {elapsed: float, delta: ~str}
{
	fn tm_to_secs(time: tm) -> float
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
		#fmt["%? %s", value, units]
	}
	
	let current = now();
	let time_secs = tm_to_secs(time);
	let current_secs = tm_to_secs(current);
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
		{elapsed: elapsed, delta: prefix + #fmt["%02d:%02d", time.tm_hour as int, time.tm_min as int]}
	}
	else
	{
		// "Thu Jan  1 00:00:00 1970"
		{elapsed: elapsed, delta: time.ctime()}
	}
}

// ----------------------------------------------------------------------------
fn run_command(tool: ~str, args: ~[~str]) -> option::option<~str>
{
	alt core::run::program_output(tool, args)
	{
		{status: 0, _}
		{
			option::none
		}
		{status: code, out: _, err: ~""}
		{
			option::some(#fmt["result code was %?", code])
		}
		{status: code, out: _, err: err}
		{
			option::some(#fmt["result code was %? (%s)", code, err])
		}
	}
}
