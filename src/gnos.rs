import io;
import io::writer_util;
import std::getopts::*;
import std::json;
import std::map::hashmap;
import core::option::extensions;
import server = rwebserve;
import model::*;
import handlers::*;

/// Various options derived from the command line and the network.json file.
type options = {
	// these are from the command line
	root: str,
	admin: bool,
	
	// these are from the network.json file
	// TODO: probably should also have device names (could then do an alert if no info for a device)
	client: str,
	server: str,
	port: u16,
	
	// this is from main
	cleanup: ~[task_runner::exit_fn],
};

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
	io::println("./gnos [options] --root=<dir> network.json");
	io::println("--admin      allows web clients to shut the server down");
	io::println("-h, --help   prints this message and exits");
	io::println("--root=DIR   path to the directory containing html files");
	io::println("--version    prints the gnos version number and exits");
	io::println("");
	io::println("--address may appear multiple times");
}

fn get_network_str(path: str, data: std::map::hashmap<str, json::json>, key: str) -> str
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

fn get_network_u16(path: str, data: std::map::hashmap<str, json::json>, key: str) -> u16
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

fn load_network_file(path: str) -> {client: str, server: str, port: u16}
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
						client: get_network_str(path, data, "client"),
						server: get_network_str(path, data, "server"),
						port: get_network_u16(path, data, "port")
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
		libc::exit(0);
	}
	else if opt_present(match, "version")
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
		root: opt_str(match, "root"),
		admin: opt_present(match, "admin"),
		
		client: network.client,
		server: network.server,
		port: network.port,
		
		cleanup: ~[],		// set in main
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

fn copy_scripts(root: str, user: str, host: str) -> option::option<str>
{
	let dir = core::path::dirname(root);						// gnos/html => /gnos
	let dir = core::path::connect(dir, "scripts");				// /gnos => /gnos/scripts
	let files = utils::list_dir_path(dir, ~[".json", ".py"]);
	
	utils::scp_files(files, user, host)
}

fn run_snmp(user: str, host: str) -> option::option<str>
{
	utils::run_remote_command(user, host, "python snmp-modeler.py -vv sat.json")
}

fn snmp_restarted(err: str, state_chan: comm::chan<msg>)
{
	let mesg = #fmt["snmp-modeler.py failed: %s", err];
	#error["%s", mesg];
	comm::send(state_chan, update_msg("alerts", |store, _err| {add_alert(store, error_level, mesg)}, err));
}

fn setup(options: options, state_chan: comm::chan<msg>) 
{
	let root = options.root;
	let client = options.client;
	let cleanup = copy options.cleanup;
	
	let action: task_runner::job_fn = || copy_scripts(root, #env["GNOS_USER"], client);
	let cp = {action: action, policy: task_runner::exit_on_failure};
	
	let action: task_runner::job_fn = || run_snmp(#env["GNOS_USER"], client);
	let run = {action: action, policy: task_runner::restart_on_failure(15, 4, |err| {snmp_restarted(err, state_chan)})};	// TODO: use 60s
	
	task_runner::sequence(~[cp, run], cleanup);
}

fn get_shutdown(options: options) -> !
{
	#info["received shutdown request"];
	for options.cleanup.each |f| {f()};
	libc::exit(0)
}

// TODO: get rid of this
fn greeting_view(_settings: hashmap<str, str>, request: server::request, response: server::response) -> server::response
{
	response.context.insert("user-name", mustache::str(@request.matches.get("name")));
	{template: "hello.html" with response}
}

fn main(args: [str])
{
	#info["starting up gnos"];
	if #env["GNOS_USER"].is_empty()
	{
		#error["GNOS_USER must be set to the name of a user able to ssh into the network json client."];
		libc::exit(1)
	}
	
	let options = parse_command_line(args);
	validate_options(options);
	
	let client = options.client;
	let c1: task_runner::exit_fn = || {utils::run_remote_command(#env["GNOS_USER"], client, "pgrep -f snmp-modeler.py | xargs --no-run-if-empty kill -9");};
	let options = {cleanup: ~[c1] with options};
	
	let state_chan = do task::spawn_listener |port| {manage_state(port)};
	setup(options, state_chan);
	
	let options2 = copy options;
	let options3 = copy options;
	let subjects_v: server::response_handler = |_settings, _request, response| {get_subjects::get_subjects(response)};	// need a unique pointer (bind won't work)
	let subject_v: server::response_handler = |_settings, request, response| {get_subject::get_subject(request, response)};	// need a unique pointer (bind won't work)
	let home_v: server::response_handler = |settings, request, response| {get_home::get_home(options2, state_chan, settings, request, response)};
	let modeler_p: server::response_handler = |_settings, request, response| {put_snmp::put_snmp(state_chan, request, response)};
	let query_s: server::open_sse = |_settings, request, push| {get_query::get_query(state_chan, request, push)};
	let bail_v: server::response_handler = |_settings, _request, _response| {get_shutdown(options3)};
	
	comm::send(state_chan, update_msg("alerts", |store, msg| {add_alert(store, error_level, msg)}, "bite my tail"));
	comm::send(state_chan, update_msg("alerts", |store, msg| {add_alert(store, error_level, msg)}, "bite my hand"));
	comm::send(state_chan, update_msg("alerts", |store, msg| {add_alert(store, warning_level, msg)}, "pet my head"));
	
	let config = {
		// We need to bind to the server addresses so that we receive modeler PUTs.
		// We bind to localhost to ensure that we can hit the web server using a local
		// browser.
		hosts: if options.admin {~[options.server, "localhost"]} else {~[options.server]},
		port: options.port,
		server_info: "gnos " + get_version(),
		resources_root: options.root,
		routes: [
			("GET", "/", "home"),
			("GET", "/shutdown", "shutdown"),		// TODO: enable this via debug cfg (or maybe via a command line option)
			("GET", "/hello/{name}", "greeting"),
			("GET", "/model", "subjects"),
			("GET", "/subject/{subject}", "subject"),
			("PUT", "/snmp-modeler", "modeler")],
		views: [
			("home",  home_v),
			("shutdown",  bail_v),
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
