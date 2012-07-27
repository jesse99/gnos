import io;
import io::writer_util;
import std::json;
import std::map::hashmap;
import core::option::extensions;
import server = rwebserve;
import model;
import options;
import handlers::*;

fn copy_scripts(root: ~str, user: ~str, host: ~str) -> option::option<~str>
{
	let dir = core::path::dirname(root);							// gnos/html => /gnos
	let dir = core::path::connect(dir, ~"scripts");				// /gnos => /gnos/scripts
	let files = utils::list_dir_path(dir, ~[~".json", ~".py"]);
	
	utils::scp_files(files, user, host)
}

fn run_snmp(user: ~str, host: ~str) -> option::option<~str>
{
	utils::run_remote_command(user, host, ~"python snmp-modeler.py -vv sat.json")
}

fn snmp_exited(err: option::option<~str>, state_chan: comm::chan<model::msg>)
{
	let mesg = #fmt["snmp-modeler.py exited%s", if err.is_some() {~" with error: " + err.get()} else {~""}];
	#error["%s", mesg];
	
	let alert = {device: ~"server", id: ~"snmp-modeler.py exited", level: model::error_level, mesg: mesg, resolution: ~"Restart gnos."};	// TODO: probably should have a button somewhere to restart the script (would have to close the alert)
	comm::send(state_chan, model::update_msg(~"alerts", |store, _err| {model::open_alert(store, alert)}, ~""));
}

fn setup(options: options::options, state_chan: comm::chan<model::msg>) 
{
	let root = options.root;
	let client = options.client;
	let cleanup = copy options.cleanup;
	
	let action: task_runner::job_fn = || copy_scripts(root, #env["GNOS_USER"], client);
	let cp = {action: action, policy: task_runner::shutdown_on_failure};
	
	let action: task_runner::job_fn = || run_snmp(#env["GNOS_USER"], client);
	let run = {action: action, policy: task_runner::notify_on_exit(|err| {snmp_exited(err, state_chan)})};
	
	task_runner::sequence(~[cp, run], cleanup);
}

fn get_shutdown(options: options::options) -> !
{
	#info["received shutdown request"];
	for options.cleanup.each |f| {f()};
	libc::exit(0)
}

fn main(args: ~[~str])
{
	#info["starting up gnos"];
	if #env["GNOS_USER"].is_empty()
	{
		#error["GNOS_USER must be set to the name of a user able to ssh into the network json client."];
		libc::exit(1)
	}
	
	let mut options = options::parse_command_line(args);
	options::validate(options);
	
	let state_chan = do task::spawn_listener |port| {model::manage_state(port)};
	if !options.db
	{
		let client = options.client;
		let c1: task_runner::exit_fn = || {utils::run_remote_command(#env["GNOS_USER"], client, ~"pgrep -f snmp-modeler.py | xargs --no-run-if-empty kill -9");};
		options.cleanup = ~[c1];
		
		setup(options, state_chan);
	}
	else
	{
		db::setup(state_chan);
	}
	
	let options2 = copy options;
	let options3 = copy options;
	let subjects_v: server::response_handler = |_settings, _request, response| {get_subjects::get_subjects(response, state_chan)};
	let subject_v: server::response_handler = |_settings, request, response| {get_subject::get_subject(request, response)};	
	let map_v: server::response_handler = |_settings, _request, response| {get_map::get_map(options2, response)};
	let modeler_p: server::response_handler = |_settings, request, response| {put_snmp::put_snmp(state_chan, request, response)};
	let query_s: server::open_sse = |_settings, request, push| {get_query::get_query(state_chan, request, push)};
	let bail_v: server::response_handler = |_settings, _request, _response| {get_shutdown(options3)};
	
	comm::send(state_chan, model::update_msg(~"alerts", |store, _msg| {model::open_alert(store, {device: ~"server", id: ~"tail", level: model::error_level, mesg: ~"bite my tail", resolution: ~"Stop biting!"})}, ~""));
	comm::send(state_chan, model::update_msg(~"alerts", |store, _msg| {model::open_alert(store, {device: ~"server", id: ~"hand", level: model::error_level, mesg: ~"bite my hand", resolution: ~"Don't bite!"})}, ~""));
	comm::send(state_chan, model::update_msg(~"alerts", |store, _msg| {model::open_alert(store, {device: ~"server", id: ~"head", level: model::warning_level, mesg: ~"pet my head", resolution: ~"Why stop?"})}, ~""));
	comm::send(state_chan, model::update_msg(~"alerts", |store, _msg| {model::open_alert(store, {device: ~"server", id: ~"content", level: model::warning_level, mesg: ~"unexplored content", resolution: ~"Visit the subjects page."})}, ~""));
	
	comm::send(state_chan, model::update_msg(~"alerts", |store, _msg| {model::open_alert(store, {device: ~"server", id: ~"content", level: model::warning_level, mesg: ~"unexplored content", resolution: ~"Visit the subjects page."})}, ~""));
	
	let config = {
		// We need to bind to the server addresses so that we receive modeler PUTs.
		// We bind to localhost to ensure that we can hit the web server using a local
		// browser.
		hosts: if options.admin {~[options.server, ~"localhost"]} else {~[options.server]},
		port: options.port,
		server_info: ~"gnos " + options::get_version(),
		resources_root: options.root,
		routes: ~[
			(~"GET", ~"/", ~"map"),
			(~"GET", ~"/shutdown", ~"shutdown"),		// TODO: enable this via debug cfg (or maybe via a command line option)
			(~"GET", ~"/model", ~"subjects"),
			(~"GET", ~"/subject/{subject}", ~"subject"),
			(~"PUT", ~"/snmp-modeler", ~"modeler")],
		views: ~[
			(~"map",  map_v),
			(~"shutdown",  bail_v),
			(~"subjects",  subjects_v),
			(~"subject",  subject_v),
			(~"modeler",  modeler_p)],
		sse: ~[(~"/query", query_s)],
		settings: ~[(~"debug",  ~"true")]		// TODO: make this a command-line option
		with server::initialize_config()};
	server::start(config);
	
	#info["exiting gnos"];						// won't normally land here
}
