//! Command line options processing.
use io::WriterUtil;
use Path = path::Path;
use std::getopts::*;

pub struct Device
{
	pub name: ~str,
	pub managed_ip: ~str,
	pub modeler: ~str,
}

/// Various options derived from the command line and the network.json file.
pub struct Options
{
	// these are from the command line
	pub root: Path,		// points to the html directory
	pub admin: bool,
	pub network_file: ~str,
	pub db: bool,
	pub browse: ~str,
	
	// these are from the network.json file
	pub network_name: ~str,
	pub client: ~str,
	pub server: ~str,
	pub port: u16,
	pub poll_rate: u16,
	pub devices: ~[Device],
}

// str constants aren't supported yet.
// TODO: get this (somehow) from the link attribute in the rc file (going the other way
// doesn't work because vers in the link attribute has to be a literal)
pub pure fn get_version() -> ~str
{
	~"0.1"
}

pub fn parse_command_line(args: ~[~str]) -> Options
{
	// It's good practice to do this before invoking getopts because getopts
	// will fail if a required option is missing.
	if args.contains(&~"-h") || args.contains(&~"--help")
	{
		print_usage();
		libc::exit(0);
	}
	
	let opts = ~[
		optflag(~"admin"),
		optflag(~"db"),			// TODO: maybe only include this if debug (in the future may also want to take a path to a turtle file)
		reqopt(~"root"),
		optflag(~"h"),
		optflag(~"help"),
		optopt(~"port"),
		optopt(~"browse"),		// TODO: not sure we always want to have this, maybe debug only?
		optflag(~"version")
	];
	let matched = match getopts(vec::tail(args), opts)
	{
		result::Ok(copy m) => {m}
		result::Err(copy f) => {io::stderr().write_line(fail_str(f)); libc::exit(1_i32)}
	};
	if opt_present(copy matched, ~"version")
	{
		io::println(fmt!("gnos %s", get_version()));
		libc::exit(0);
	}
	else if matched.free.len() != 1
	{
		io::stderr().write_line("Expected one positional argument: a network json file.");
		libc::exit(1);
	}
	
	let path: path::Path = path::from_str(matched.free[0]);
	let network = load_network_file(&path);
	
	Options
	{
		root: path::from_str(opt_str(copy matched, ~"root")),
		admin: opt_present(copy matched, ~"admin"),
		network_file: path.filename().get(),
		db: opt_present(copy matched, ~"db"),
		browse: opt_str(copy matched, ~"browse"),
		
		network_name: network.network,
		client: network.client,
		server: if opt_present(copy matched, ~"db") {~"localhost"} else {copy network.server},
		port: network.port,
		poll_rate: network.poll_rate,
		devices: network.devices,
	}
}

pub fn validate(options: &Options)
{
	if !os::path_is_dir(&options.root)
	{
		io::stderr().write_line(fmt!("'%s' does not point to a directory.", options.root.to_str()));
		libc::exit(1_i32);
	}
}

// ---- Internal Functions ----------------------------------------------------
priv fn print_usage()
{
	io::println(fmt!("gnos %s - a web based network management system", get_version()));
	io::println(~"");
	io::println(~"./gnos [options] --root=DIR network.json");
	io::println(~"--admin     allows web clients to shut the server down");
	io::println(~"--db        use a hard-coded database instead of modeler scripts");
	io::println(~"-h, --help  prints this message and exits");
	io::println(~"--root=DIR  path to the directory containing html files");
	io::println(~"--version   prints the gnos version number and exits");
}

priv fn load_network_file(path: &Path) -> {network: ~str, client: ~str, server: ~str, port: u16, poll_rate: u16, devices: ~[Device]}
{
	match io::file_reader(path)
	{
		result::Ok(reader) =>
		{
			match std::json::from_reader(reader)
			{
				result::Ok(std::json::Object(ref data)) =>
				{
					{
						network: get_network_str(path, *data, &~"network"),
						client: get_network_str(path, *data, &~"client"),
						server: get_network_str(path, *data, &~"server"),
						port: get_network_u16(path, *data, &~"port"),
						poll_rate: get_network_u16(path, *data, &~"poll-rate"),
						devices: get_network_devices(path, *data, &~"devices"),
					}
				}
				result::Ok(ref x) =>
				{
					io::stderr().write_line(fmt!("Error parsing '%s': expected json::dict but found %?.", path.to_str(), x));
					libc::exit(1)
				}
				result::Err(err) =>
				{
					io::stderr().write_line(fmt!("Error parsing '%s' on line %?: %s.", path.to_str(), err.line, *err.msg));
					libc::exit(1)
				}
			}
		}
		result::Err(ref err) =>
		{
			io::stderr().write_line(fmt!("Error reading '%s': %s.", path.to_str(), *err));
			libc::exit(1)
		}
	}
}

priv fn get_network_devices(path: &Path, data: &send_map::linear::LinearMap<~str, std::json::Json>, key: &~str) -> ~[Device]
{
	match data.find(key)
	{
		option::Some(std::json::Object(ref value)) =>
		{
			let mut devices = ~[];
			for value.each
			|key, value|
			{
				vec::push(&mut devices, get_network_device(path, *key, value));
			}
			devices
		}
		option::Some(ref x) =>
		{
			io::stderr().write_line(fmt!("In '%s' %s was expected to be a json::dict but was %?.", path.to_str(), *key, x));
			libc::exit(1)
		}
		option::None =>
		{
			io::stderr().write_line(fmt!("Expected to find %s in '%s'.", *key, path.to_str()));
			libc::exit(1)
		}
	}
}

priv fn get_network_device(path: &Path, name: &str, value: &std::json::Json) -> Device
{
	match *value
	{
		std::json::Object(ref value) =>
		{
			Device {
				name: name.to_unique(),
				managed_ip: get_network_str(path, *value, &~"ip"),
				modeler: get_network_str(path, *value, &~"modeler"),
			}
		}
		ref x =>
		{
			io::stderr().write_line(fmt!("In '%s' %s was expected to be a json::dict but was %?.", path.to_str(), name, x));
			libc::exit(1)
		}
	}
}

priv fn get_network_str(path: &Path, data: &send_map::linear::LinearMap<~str, std::json::Json>, key: &~str) -> ~str
{
	match data.find(key)
	{
		option::Some(std::json::String(ref value)) =>
		{
			value.to_unique()
		}
		option::Some(ref x) =>
		{
			io::stderr().write_line(fmt!("In '%s' %s was expected to be a json::string but was %?.", path.to_str(), *key, x));
			libc::exit(1)
		}
		option::None =>
		{
			io::stderr().write_line(fmt!("Expected to find %s in '%s'.", *key, path.to_str()));
			libc::exit(1)
		}
	}
}

priv fn get_network_u16(path: &Path, data: &send_map::linear::LinearMap<~str, std::json::Json>, key: &~str) -> u16
{
	match data.find(key)
	{
		option::Some(std::json::Number(value)) =>
		{
			if value > u16::max_value as float
			{
				io::stderr().write_line(fmt!("In '%s' %s was too large for a u16.", path.to_str(), *key));
				libc::exit(1);
			}
			if value < 0.0
			{
				io::stderr().write_line(fmt!("In '%s' %s was negative.", path.to_str(), *key));
				libc::exit(1);
			}
			value as u16
		}
		option::Some(ref x) =>
		{
			io::stderr().write_line(fmt!("In '%s' %s was expected to be a json::num but was %?.", path.to_str(), *key, x));
			libc::exit(1)
		}
		option::None =>
		{
			io::stderr().write_line(fmt!("Expected to find %s in '%s'.", *key, path.to_str()));
			libc::exit(1)
		}
	}
}

priv fn get_network_float(path: &Path, data: &send_map::linear::LinearMap<~str, std::json::Json>, key: &~str) -> float
{
	match data.find(key)
	{
		option::Some(std::json::Number(value)) =>
		{
			value
		}
		option::Some(ref x) =>
		{
			io::stderr().write_line(fmt!("In '%s' %s was expected to be a json::num but was %?.", path.to_str(), *key, x));
			libc::exit(1)
		}
		option::None =>
		{
			io::stderr().write_line(fmt!("Expected to find %s in '%s'.", *key, path.to_str()));
			libc::exit(1)
		}
	}
}
