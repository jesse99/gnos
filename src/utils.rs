export scp_files, run_remote_command, list_dir_path;

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
