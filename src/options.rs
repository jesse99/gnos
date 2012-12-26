//! Command line options processing.
use core::path::{GenericPath};
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
	pub bind_ip: ~str,
	pub bind_port: u16,
	
	// these are from the network.json file
	pub network_name: ~str,
	pub client_ip: ~str,
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
		optopt(~"bind"),
		optopt(~"browse"),		// TODO: not sure we always want to have this, maybe debug only?
		optflag(~"version")
	];
	let matched = match getopts(vec::tail(args), opts)
	{
		result::Ok(copy m) => {m}
		result::Err(copy f) => {io::stderr().write_line(fail_str(f)); libc::exit(1_i32)}
	};
	if opt_present(&matched, ~"version")
	{
		io::println(fmt!("gnos %s", get_version()));
		libc::exit(0);
	}
	else if matched.free.len() != 1
	{
		io::stderr().write_line("Expected one positional argument: a network json file.");
		libc::exit(1);
	}
	
	let path: path::Path = GenericPath::from_str(matched.free[0]);
	let network = load_network_file(&path);
	
	Options
	{
		root: GenericPath::from_str(opt_str(&matched, ~"root")),
		admin: opt_present(&matched, ~"admin"),
		network_file: path.filename().get(),
		db: opt_present(&matched, ~"db"),
		browse: if opt_present(&matched, ~"browse") {opt_str(&matched, ~"browse")} else {~""},
		bind_ip: if opt_present(&matched, ~"bind") {endpoint_to_ip(opt_str(&matched, ~"bind"))} else {~"127.0.0.1"},
		bind_port: if opt_present(&matched, ~"bind") {endpoint_to_port(opt_str(&matched, ~"bind"))} else {8080},
		
		network_name: copy network.network,
		client_ip: copy network.client,
		poll_rate: network.poll_rate,
		devices: copy network.devices,
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
	io::println(~"--bind=IP[:PORT]  local address or interface to bind to [127.0.0.1:8080");
	io::println(~"--browse=URL  use git to open a browser window to the URL");
	io::println(~"--db        use a hard-coded database instead of modeler scripts");
	io::println(~"-h, --help  prints this message and exits");
	io::println(~"--root=DIR  path to the directory containing html files");
	io::println(~"--version   prints the gnos version number and exits");
}

priv fn load_network_file(path: &Path) -> {network: ~str, client: ~str, poll_rate: u16, devices: ~[Device]}
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
				name: name.to_owned(),
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
			value.to_owned()
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

// TODO: This is moderately terrible. Probably the way to go is something like
// http://www.geekpage.jp/en/programming/linux-network/get-ipaddr.php
// but it might be tricky to write portable rust wrappers for that.
priv fn find_interface_ip(ifname: ~str) -> ~str
{
	// eth1.100  Link encap:Ethernet  HWaddr 00:10:18:49:e4:6d
	//     inet addr:172.16.0.2  Bcast:172.16.0.255  Mask:255.255.255.0
	//
	// utun0: flags=80d1<UP,POINTOPOINT,RUNNING,NOARP,MULTICAST> mtu 1399
	//	inet 10.6.210.108 --> 10.6.210.108 netmask 0xffff8000 
	match core::run::program_output("/sbin/ifconfig", &[copy ifname])
	{
		{status: 0, out: ref out, _} =>
		{
			let p1 = "inet addr:";
			let p2 = "inet ";
			let i1 = str::find_str(*out, p1);
			let i2 = str::find_str(*out, p2);
			if i1.is_some()
			{
				let k = str::find_char_from(*out, ' ', i1.get() + p1.len());
				out.slice(i1.get() + p1.len(), k.get())
			}
			else if i2.is_some()
			{
				let k = str::find_char_from(*out, ' ', i2.get() + p2.len());
				out.slice(i2.get() + p2.len(), k.get())
			}
			else
			{
				io::stderr().write_line(fmt!("Couldn't find '%s' or '%s' in '%s'.", p1, p2, *out));
				libc::exit(1)
			}
		}
		_ =>
		{
			io::stderr().write_line(fmt!("'%s' is not an IP address or interface name.", ifname));
			libc::exit(1)
		}
	}
}

priv fn endpoint_to_ip(endpoint: &str) -> ~str
{
	let addr = match str::find_char(endpoint, ':')
	{
		option::Some(i) => endpoint.slice(0, i),
		option::None => endpoint.to_owned(),
	};
	if str::all(addr, |c| char::is_digit(c) || c == '.')	// TODO: could do some better validation here
	{
		addr
	}
	else
	{
		find_interface_ip(addr)
	}
}

fn endpoint_to_port(endpoint: &str) -> u16
{
	let port = match str::find_char(endpoint, ':')
	{
		option::Some(i) => endpoint.slice(i + 1, endpoint.len()),
		option::None => ~"8080",
	};
	match u16::from_str(port)
	{
		option::Some(p) => p,
		option::None =>
		{
			io::stderr().write_line(fmt!("'%s' is not a valid port.", port));
			libc::exit(1)
		}
	}
}

