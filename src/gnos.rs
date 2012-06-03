import io;
import io::writer_util;
import std::json;
import std::getopts::*;
import std::map::hashmap;
import to_str::to_str;
import mustache::to_mustache;
import server = rwebserve::server;
import rrdf::*;

type options = {root: str, admin: bool, addresses: [str], port: u16};

// str constants aren't supported yet.
// TODO: get this (somehow) from the link attribute in the rc file (going the other way
// doesn't work because vers in the link attribute has to be a literal)
fn get_version() -> str
{
	"0.1"
}

fn print_usage()
{
	io::println(#fmt["gnos %s - a web based network management system", get_version()]);
	io::println("");
	io::println("./gnos [options] --root=<dir>");
	io::println("--address=IP ip address or 'localhost' to bind to [0.0.0.0]");
	io::println("--admin      allows web clients to shut the server down");
	io::println("-h, --help   prints this message and exits");
	io::println("--port=NUM   port to bind to [80]");
	io::println("--root=DIR   path to the directory containing html files");
	io::println("--version    prints the gnos version number and exits");
	io::println("");
	io::println("--address may appear multiple times");
} 

fn opt_str_or_default(match: match, name: str, default: str) -> str
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

fn opt_strs_or_default(match: match, name: str, default: [str]) -> [str]
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

fn parse_command_line(args: [str]) -> options
{
	let opts = [
		optflag("admin"),
		optmulti("address"),
		reqopt("root"),
		optflag("h"),
		optflag("help"),
		optopt("port"),
		optflag("version")
	];
	let match = alt getopts(vec::tail(args), opts)
	{
		result::ok(m) {m}
		result::err(f) {io::stderr().write_line(fail_str(f)); libc::exit(1_i32)}
	};
	if opt_present(match, "h") || opt_present(match, "help")
	{
		print_usage();
		libc::exit(0_i32);
	}
	else if opt_present(match, "version")
	{
		io::println(#fmt["gnos %s", get_version()]);
		libc::exit(0_i32);
	}
	else if vec::is_not_empty(match.free)
	{
		io::stderr().write_line("Positional arguments are not allowed.");
		libc::exit(1_i32);
	}
	let port = alt uint::from_str(opt_str_or_default(match, "port", "80"))
	{
		option::some(v)
		{
			if v > u16::max_value as uint
			{
				io::stderr().write_line("Port is too large.");
				libc::exit(1_i32);
			}
			v as u16
		}
		option::none
		{
			io::stderr().write_line("Port should be formatted as a unsigned 16-bit number.");
			libc::exit(1_i32)
		}
	};
	{
		root: opt_str(match, "root"),
		admin: opt_present(match, "admin"),
		addresses: opt_strs_or_default(match, "address", ["0.0.0.0"]),
		port: port
	}
}

fn validate_options(options: options)
{
	if !os::path_is_dir(options.root)
	{
		io::stderr().write_line(#fmt["'%s' does not point to a directory.", options.root]);
		libc::exit(1_i32);
	}
}

fn subject_view(_options: options, _settings: hashmap<str, str>, _request: server::request, response: server::response) -> server::response
{
	//let subject = request.matches.get("subject");
	//let matches = vec::filter(graph) {|elem| elem.subject == iri(subject)};
	
	//fn le(&&a: triple, &&b: triple) -> bool {a.property <= b.property}
	//let matches = std::sort::merge_sort(le, matches);
	
	//let mut properties = [];
	//for vec::eachi(matches)
	//{|index, match|
	//	let map = std::map::str_hash();
	//	let (urls, normals) = subject_utils::object_to_context(match.object);
	//	map.insert("row-class", mustache::str(if index & 1u == 0u {"even"} else {"odd"}));
	//	map.insert("property", mustache::str(match.property));
	//	map.insert("has-urls", vec::is_not_empty(urls).to_mustache());
	//	map.insert("url-objects", urls.to_mustache());
	//	map.insert("normal-objects", normals.to_mustache());
	//	vec::push(properties, mustache::map(map));
	//};
	
	//response.context.insert("subject", mustache::str(subject));
	//response.context.insert("properties", mustache::vec(properties));
	
	{template: "(private)/subject.html" with response}
}

enum msg
{
	getter(comm::chan<[triple]>),
	setter([triple])
}

fn manage_state(port: comm::port<msg>)
{
	let mut state = [];
	
	loop
	{
		alt comm::recv(port)
		{
			getter(channel)
			{
				comm::send(channel, copy(state));
			}
			setter(new_state)
			{
				state = new_state;
			}
		}
	}
}

fn home_view(options: options, state_chan: comm::chan<msg>, _settings: hashmap<str, str>, _request: server::request, response: server::response) -> server::response
{
	let port = comm::port::<[triple]>();
	let chan = comm::chan::<[triple]>(port);
	comm::send(state_chan, getter(chan));
	let state = comm::recv(port);
	
	let mut triples = [];
	for vec::each(state)
	{|triple|
		let map = std::map::str_hash();
		map.insert("subject", mustache::str(triple.subject.to_str()));
		map.insert("property", mustache::str(triple.property));
		map.insert("object", mustache::str(triple.object.to_str()));
		vec::push(triples, mustache::map(map));
	};
	
	response.context.insert("store", mustache::vec(triples));
	response.context.insert("admin", mustache::bool(options.admin));
	{template: "home.html" with response}
}

fn greeting_view(_settings: hashmap<str, str>, request: server::request, response: server::response) -> server::response
{
	response.context.insert("user-name", mustache::str(request.matches.get("name")));
	{template: "hello.html" with response}
}

fn add_string(&triples: [triple], subject: str, property: str, value: str)
{
	let object = primitive(string(value));
	vec::push(triples, {subject: iri(subject), property: property, object: object});
}

fn add_strings(&triples: [triple], subject: str, property: str, value: [str])
{
	let objects = vec::map(value) {|v| primitive(string(v))};
	vec::push(triples, {subject: iri(subject), property: property, object: seq(objects)});
}

fn add_str_default(&triples: [triple], subject: str, property: str, data: std::map::hashmap<str, std::json::json>, key: str, default: str)
{
	let object = alt data.find(key)
	{
		some(value)
		{
			alt value
			{
				std::json::string(s)
				{
					s
				}
				_
				{
					// This is something that should never happen so it's not so bad that we don't provide a lot of context
					// (if it does somehow happen admins can crank up the logging level to see where it is coming from).
					#error["%s was expected to be a string but is a %?", key, value];	// TODO: would be nice if the site could somehow show logs
					default
				}
			}
		}
		none
		{
			default
		}
	};
	add_string(triples, subject, property, object);
}

// We save the most important bits of data that we receive from json into gnos statements
// so that we can more easily model devices that don't use snmp.
fn add_device(&triples: [triple], subject: str, device: std::map::hashmap<str, std::json::json>)
{
	add_str_default(triples, subject, "gnos:name", device, "sysName", "<unknown>");	// TODO: admin property, if set, should override this
	add_str_default(triples, subject, "gnos:description", device, "sysDescr", "");			// TODO: admin property, if set, should override this
}

// We store snmp data for various objects in the raw so that views are able to use it
// and so admins can view the complete raw data.
fn add_snmp(&triples: [triple], subject: str, object: std::map::hashmap<str, std::json::json>)
{
	for object.each()
	{|name, value|
		alt value
		{
			std::json::string(s)
			{
				add_string(triples, subject, name, s);
			}
			std::json::dict(_d)
			{
				// our caller should handle this with another call to add_smp
			}
			std::json::list(_l)
			{
				// this is the interfaces list
				// TODO: can probably nuke this once we start processing the interfaces
			}
			_
			{
				#error["%s was expected to contain string, dict, and list items but had a %?", subject, value];
			}
		}
	};
}

// Format is:
// {
//    "10.101.100.2": 			managed ip address
//    {
//        "ipBlah": "23",			primitive values are all strings
//        ...
//        "interfaces": 
//        [
//            {,
//               "ifBlah": "foo",
//                ...
//            },
//            ...
//        ]
//    },
//    ...
// }
fn json_to_triples(remote_addr: str, data: std::json::json) -> [triple]
{
	let mut triples = [];
	
	// TODO: This should come from some data structure under the control of an admin.
	add_strings(triples, "gnos:admin", "gnos:managed-ips", ["10.101.100.2"]);
	
	alt data
	{
		std::json::dict(d)
		{
			for d.each()
			{|managed_ip, the_device|
				alt the_device
				{
					std::json::dict(device)
					{
						let subject = #fmt["gnos:device-%s", managed_ip];
						add_device(triples, subject, device);
						
						let subject = #fmt["gnos:snmp-device-%s", managed_ip];
						add_snmp(triples, subject, device);
					}
					_
					{
						#error["%s device from %s was expected to be a dict but is a %?", managed_ip, remote_addr, the_device];	// TODO: probably want to add errors to store
					}
				}
			};
		}
		_
		{
			#error["Data from %s was expected to be a dict but is a %?", remote_addr, data];	// TODO: probably want to add errors to store
		}
	}
	
	#debug["Data received from %s:", remote_addr];
	for vec::each(triples)
	{|triple|
		#debug["   %s", triple.to_str()];
	};
	
	ret triples;
}

fn modeler_put(state_chan: comm::chan<msg>, request: server::request, response: server::response) -> server::response
{
	// TODO:
	// have the home page show something about the json
	// test the error case
	
	// may want to verify that the json is a dict
	// convert the json into triples
	// send the triples to the state manager task
	// change home so that it shows the devices in the store
	// home should have (in admin/debug) a metric for the store size
	// commit and push rwebserve changes
	
	alt std::json::from_str(request.body)
	{
		result::ok(data)
		{
			#info["got new modeler data"];
			comm::send(state_chan, setter(json_to_triples(request.remote_addr, data)));
			{body: "" with response}
		}
		result::err(err)
		{
			let intro = #fmt["Malformed json on line %? col %?", err.line, err.col];
			#info["Error getting new modeler data:"];
			#info["%s: %s", intro, err.msg];
			
			response.context.insert("intro", mustache::str(intro));
			response.context.insert("error", mustache::str(err.msg));
			{status: "400 Bad Request", template: "bad-request" with response}	// not terribly useful to send html to scripts, but it might be handy to have the error visible in packets
		}
	}
}

// TODO: Will it be too expensive to copy the entire state? Do we need to support partial updates
// and queries?
fn main(args: [str])
{
	#info["starting up gnos"];
	let options = parse_command_line(args);
	validate_options(options);
	
	let state_chan = task::spawn_listener {|port| manage_state(port)};
	
	let subject_v: server::response_handler = {|settings, request, response| subject_view(options, settings, request, response)};	// need a unique pointer (bind won't work)
	let home_v: server::response_handler = {|settings, request, response| home_view(options, state_chan, settings, request, response)};
	let modeler_p: server::response_handler = {|_settings, request, response| modeler_put(state_chan, request, response)};
	
	let config = {
		hosts: options.addresses,
		port: options.port,
		server_info: "gnos " + get_version(),
		resources_root: options.root,
		routes: [
			("GET", "/", "home"),
			("GET", "/hello/{name}", "greeting"),
			("GET", "/subject/{subject}", "subject"),
			("PUT", "/snmp-modeler", "modeler")],
		views: [
			("home",  home_v),
			("greeting", greeting_view),
			("subject",  subject_v),
			("modeler",  modeler_p)],
		settings: [("debug",  "true")]			// TODO: make this a command-line option
		with server::initialize_config()};
	server::start(config);
	#info["exiting gnos"];						// won't normally land here
}
