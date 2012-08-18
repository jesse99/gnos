import std::getopts::*;

export scp_files, run_remote_command, list_dir_path, imprecise_time_s, i64_to_unit_str;

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

// Takes an integer value and returns a string like "234", "200K", "3.2M", etc.
fn i64_to_unit_str(value: i64) -> ~str
{
	if value < 1024
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
