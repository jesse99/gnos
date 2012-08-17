//! Command line options processing.
import std::getopts::*;

export options, device, get_version, validate, parse_command_line;

type device = {name: ~str, managed_ip: ~str, community: ~str, center_x: float, center_y: float, style: ~str};

/// Various options derived from the command line and the network.json file.
type options =
{
	// these are from the command line
	root: ~str,
	admin: bool,
	script: ~str,
	db: bool,
	
	// these are from the network.json file
	client: ~str,
	server: ~str,
	port: u16,
	poll_rate: u16,
	devices: ~[device],
	
	// this is from main
	cleanup: ~[task_runner::exit_fn],
};

// str constants aren't supported yet.
// TODO: get this (somehow) from the link attribute in the rc file (going the other way
// doesn't work because vers in the link attribute has to be a literal)
fn get_version() -> ~str
{
	~"0.1"
}

fn parse_command_line(args: ~[~str]) -> options
{
	let opts = ~[
		optflag(~"admin"),
		optflag(~"db"),			// TODO: maybe only include this if debug (in the future may also want to take a path to a turtle file)
		reqopt(~"root"),
		optflag(~"h"),
		optflag(~"help"),
		optopt(~"port"),
		optflag(~"version")
	];
	let match = alt getopts(vec::tail(args), opts)
	{
		result::ok(m) {m}
		result::err(f) {io::stderr().write_line(fail_str(f)); libc::exit(1_i32)}
	};
	if opt_present(match, ~"h") || opt_present(match, ~"help")
	{
		print_usage();
		libc::exit(0);
	}
	else if opt_present(match, ~"version")
	{
		io::println(#fmt["gnos %s", get_version()]);
		libc::exit(0);
	}
	else if match.free.len() != 1
	{
		io::stderr().write_line("Expected one positional argument: a network json file.");
		libc::exit(1);
	}
	let network = load_network_file(match.free[0]);
	
	{
		root: opt_str(match, ~"root"),
		admin: opt_present(match, ~"admin"),
		script: path::basename(match.free[0]),
		db: opt_present(match, ~"db"),
		
		client: network.client,
		server: network.server,
		port: network.port,
		poll_rate: network.poll_rate,
		devices: network.devices,
		
		cleanup: ~[],		// set in main
	}
}

fn validate(options: options)
{
	if !os::path_is_dir(options.root)
	{
		io::stderr().write_line(#fmt["'%s' does not point to a directory.", options.root]);
		libc::exit(1_i32);
	}
}

// ---- Internal Functions ----------------------------------------------------
fn print_usage()
{
	io::println(#fmt["gnos %s - a web based network management system", get_version()]);
	io::println(~"");
	io::println(~"./gnos [options] --root=DIR network.json");
	io::println(~"--admin     allows web clients to shut the server down");
	io::println(~"--db        use a hard-coded database instead of modeler scripts");
	io::println(~"-h, --help  prints this message and exits");
	io::println(~"--root=DIR  path to the directory containing html files");
	io::println(~"--version   prints the gnos version number and exits");
}

fn load_network_file(path: ~str) -> {client: ~str, server: ~str, port: u16, poll_rate: u16, devices: ~[device]}
{
	alt io::file_reader(path)
	{
		result::ok(reader)
		{
			alt json::from_reader(reader)
			{
				result::ok(json::dict(data))
				{
					{
						client: get_network_str(path, data, ~"client"),
						server: get_network_str(path, data, ~"server"),
						port: get_network_u16(path, data, ~"port"),
						poll_rate: get_network_u16(path, data, ~"poll-rate"),
						devices: get_network_devices(path, data, ~"devices"),
					}
				}
				result::ok(x)
				{
					io::stderr().write_line(#fmt["Error parsing '%s': expected json::dict but found %?.", path, x]);
					libc::exit(1)
				}
				result::err(err)
				{
					io::stderr().write_line(#fmt["Error parsing '%s' on line %?: %s.", path, err.line, *err.msg]);
					libc::exit(1)
				}
			}
		}
		result::err(err)
		{
			io::stderr().write_line(#fmt["Error reading '%s': %s.", path, err]);
			libc::exit(1)
		}
	}
}

fn get_network_devices(path: ~str, data: std::map::hashmap<~str, json::json>, key: ~str) -> ~[device]
{
	alt data.find(key)
	{
		option::some(json::dict(value))
		{
			let mut devices = ~[];
			for value.each
			|key, value|
			{
				vec::push(devices, get_network_device(path, key, value));
			}
			devices
		}
		option::some(x)
		{
			io::stderr().write_line(#fmt["In '%s' %s was expected to be a json::dict but was %?.", path, key, x]);
			libc::exit(1)
		}
		option::none
		{
			io::stderr().write_line(#fmt["Expected to find %s in '%s'.", key, path]);
			libc::exit(1)
		}
	}
}

fn get_network_device(path: ~str, name: ~str, value: json::json) -> device
{
	alt value
	{
		json::dict(value)
		{
			{
				name: name,
				managed_ip: get_network_str(path, value, ~"ip"),
				community: get_network_str(path, value, ~"community"),
				center_x: get_network_float(path, value, ~"center_x"),
				center_y: get_network_float(path, value, ~"center_y"),
				style: get_network_str(path, value, ~"type"),
			}
		}
		x
		{
			io::stderr().write_line(#fmt["In '%s' %s was expected to be a json::dict but was %?.", path, name, x]);
			libc::exit(1)
		}
	}
}

fn get_network_str(path: ~str, data: std::map::hashmap<~str, json::json>, key: ~str) -> ~str
{
	alt data.find(key)
	{
		option::some(json::string(value))
		{
			*value
		}
		option::some(x)
		{
			io::stderr().write_line(#fmt["In '%s' %s was expected to be a json::string but was %?.", path, key, x]);
			libc::exit(1)
		}
		option::none
		{
			io::stderr().write_line(#fmt["Expected to find %s in '%s'.", key, path]);
			libc::exit(1)
		}
	}
}

fn get_network_u16(path: ~str, data: std::map::hashmap<~str, json::json>, key: ~str) -> u16
{
	alt data.find(key)
	{
		option::some(json::num(value))
		{
			if value > u16::max_value as float
			{
				io::stderr().write_line(#fmt["In '%s' %s was too large for a u16.", path, key]);
				libc::exit(1);
			}
			if value < 0.0
			{
				io::stderr().write_line(#fmt["In '%s' %s was negative.", path, key]);
				libc::exit(1);
			}
			value as u16
		}
		option::some(x)
		{
			io::stderr().write_line(#fmt["In '%s' %s was expected to be a json::num but was %?.", path, key, x]);
			libc::exit(1)
		}
		option::none
		{
			io::stderr().write_line(#fmt["Expected to find %s in '%s'.", key, path]);
			libc::exit(1)
		}
	}
}

fn get_network_float(path: ~str, data: std::map::hashmap<~str, json::json>, key: ~str) -> float
{
	alt data.find(key)
	{
		option::some(json::num(value))
		{
			value
		}
		option::some(x)
		{
			io::stderr().write_line(#fmt["In '%s' %s was expected to be a json::num but was %?.", path, key, x]);
			libc::exit(1)
		}
		option::none
		{
			io::stderr().write_line(#fmt["Expected to find %s in '%s'.", key, path]);
			libc::exit(1)
		}
	}
}
