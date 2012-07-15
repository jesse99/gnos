import io;
import io::writer_util;
import std::getopts::*;
import std::map::hashmap;
import server = rwebserve;
import model::*;
import handlers::*;

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

// TODO: get rid of this
fn greeting_view(_settings: hashmap<str, str>, request: server::request, response: server::response) -> server::response
{
	response.context.insert("user-name", mustache::str(@request.matches.get("name")));
	{template: "hello.html" with response}
}

// TODO: Will it be too expensive to copy the entire state? Do we need to support partial updates
// and queries?
fn main(args: [str])
{
	#info["starting up gnos"];
	let options = parse_command_line(args);
	validate_options(options);
	
	let state_chan = do task::spawn_listener |port| {manage_state(port)};
	
	let subjects_v: server::response_handler = |_settings, _request, response| {get_subjects::get_subjects(response)};	// need a unique pointer (bind won't work)
	let subject_v: server::response_handler = |_settings, request, response| {get_subject::get_subject(request, response)};	// need a unique pointer (bind won't work)
	let home_v: server::response_handler = |settings, request, response| {get_home::get_home(options, state_chan, settings, request, response)};
	let modeler_p: server::response_handler = |_settings, request, response| {put_snmp::put_snmp(state_chan, request, response)};
	let query_s: server::open_sse = |_settings, request, push| {get_query::get_query(state_chan, request, push)};
	
	let config = {
		hosts: options.addresses,
		port: options.port,
		server_info: "gnos " + get_version(),
		resources_root: options.root,
		routes: [
			("GET", "/", "home"),
			("GET", "/hello/{name}", "greeting"),
			("GET", "/model", "subjects"),
			("GET", "/subject/{subject}", "subject"),
			("PUT", "/snmp-modeler", "modeler")],
		views: [
			("home",  home_v),
			("greeting", greeting_view),
			("subjects",  subjects_v),
			("subject",  subject_v),
			("modeler",  modeler_p)],
		sse: ~[("/query", query_s)],
		settings: [("debug",  "true")]			// TODO: make this a command-line option
		with server::initialize_config()};
	server::start(config);

	#info["exiting gnos"];						// won't normally land here
}
