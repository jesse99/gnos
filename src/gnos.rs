use Path = path::Path;
use io::WriterUtil;
use std::json;
use std::map::*;
use server = rwebserve::rwebserve;
use handlers::*;
use rrdf::rrdf::*;
use task_runner::*;

fn copy_scripts(root: Path, user: ~str, host: ~str) -> option::Option<~str>
{
	let dir = core::os::make_absolute(&root).pop();	// gnos/html => /gnos
	let dir = dir.push(~"scripts");						// /gnos => /gnos/scripts
	let files = utils::list_dir_path(&dir, ~[~".json", ~".py"]);
	
	utils::scp_files(files, user, host)
}

fn run_snmp(user: ~str, host: ~str, script: ~str) -> option::Option<~str>
{
	utils::run_remote_command(user, host, ~"python snmp-modeler.py -vv " + script)
}

fn snmp_exited(err: option::Option<~str>, state_chan: comm::Chan<model::Msg>)
{
	let mesg = fmt!("snmp-modeler.py exited%s", if err.is_some() {~" with error: " + err.get()} else {~""});
	error!("%s", mesg);
	
	let alert = model::Alert {device: ~"gnos:map", id: ~"snmp-modeler.py exited", level: model::ErrorLevel, mesg: mesg, resolution: ~"Restart gnos."};	// TODO: probably should have a button somewhere to restart the script (would have to close the alert)
	comm::send(state_chan, model::UpdateMsg(~"alerts", |store, _err| {model::open_alert(store, alert)}, ~""));
}

fn setup(options: options::Options, state_chan: comm::Chan<model::Msg>) 
{
	let root = options.root;
	let client2 = copy options.client;
	let cleanup = copy options.cleanup;
	
	let action: task_runner::JobFn = || copy_scripts(root, env!("GNOS_USER"), client2);
	let cp = Job {action: action, policy: task_runner::ShutdownOnFailure};
	
	let client3 = copy options.client;
	let ccc = copy(options.script);
	let action: task_runner::JobFn = || run_snmp(env!("GNOS_USER"), client3, ccc);
	let run = Job {action: action, policy: task_runner::NotifyOnExit(|err| {snmp_exited(err, state_chan)})};
	
	task_runner::sequence(~[cp, run], cleanup);
}

fn get_shutdown(options: options::Options) -> !
{
	info!("received shutdown request");
	for options.cleanup.each |f| {f()};
	libc::exit(0)
}

fn update_globals(store: &Store, options: options::Options) -> bool
{
	store.add(~"gnos:globals", ~[
		(~"gnos:admin", BoolValue(true)),		// TODO: get this from a setting
		(~"gnos:debug", BoolValue(true)),		// TODO: get this from command line
	]);
	
	let devices = vec::zip(vec::from_elem(options.devices.len(), ~"gnos:device"), do options.devices.map |n| {StringValue(n.managed_ip, ~"")});
	store.add(~"gnos:globals", devices);
	
	let names = model::get_standard_store_names();
	let stores = vec::zip(vec::from_elem(names.len(), ~"gnos:store"), do names.map |n| {StringValue(n, ~"")});
	store.add(~"gnos:globals", stores);
	
	true
}

fn main(args: ~[~str])
{
	info!("starting up gnos");
	if env!("GNOS_USER").is_empty()
	{
		error!("GNOS_USER must be set to the name of a user able to ssh into the network json client.");
		libc::exit(1)
	}
	
	let mut options = options::parse_command_line(args);
	options::validate(options);
	
	let state_chan = do utils::spawn_moded_listener(task::ManualThreads(2)) |port| {model::manage_state(port)};
	if !options.db
	{
		let client = options.client;
		let c1: task_runner::ExitFn = || {utils::run_remote_command(env!("GNOS_USER"), client, ~"pgrep -f snmp-modeler.py | xargs --no-run-if-empty kill -9");};
		options.cleanup = ~[c1];
		
		setup(options, state_chan);
	}
	else
	{
		db::setup(state_chan, options.poll_rate);
	}
	
	let options1 = copy options;
	comm::send(state_chan, model::UpdateMsg(~"globals", |store, _err| {update_globals(store, options1)}, ~""));
	
	let options2 = copy options;
	let options3 = copy options;
	let options4 = copy options;
	let options5 = copy options;
	let options6 = copy options;
	let options7 = copy options;
	let models_v: server::ResponseHandler = |_settings, _request: &server::Request, response: &server::Response| {get_models::get_models(options5, response, state_chan)};
	let subject_v: server::ResponseHandler = |_settings, request: &server::Request, response: &server::Response| {get_subject::get_subject(options6, request, response)};
	let map_v: server::ResponseHandler = |_settings, _request: &server::Request, response: &server::Response| {get_map::get_map(options2, response)};
	let modeler_p: server::ResponseHandler = |_settings, request: &server::Request, response: &server::Response| {put_snmp::put_snmp(options4, state_chan, request, response)};
	let query_store_v: server::ResponseHandler = |_settings, request: &server::Request, response: &server::Response| {get_query_store::get_query_store(options7, request, response)};
	let query_s: server::OpenSse = |_settings, request: &server::Request, push| {get_query::get_query(state_chan, request, push)};
	let bail_v: server::ResponseHandler = |_settings, _request: &server::Request, _response: &server::Response| {get_shutdown(options3)};
	
	let config = server::Config
	{
		// We need to bind to the server addresses so that we receive modeler PUTs.
		// We bind to localhost to ensure that we can hit the web server using a local
		// browser.
		hosts: if options.db {~[~"localhost"]} else if options.admin {~[options.server, ~"localhost"]} else {~[options.server]},
		port: options.port,
		server_info: ~"gnos " + options::get_version(),
		resources_root: options.root,
		routes: ~[
			(~"GET", ~"/", ~"map"),
			(~"GET", ~"/shutdown", ~"shutdown"),		// TODO: enable this via debug cfg (or maybe via a command line option)
			(~"GET", ~"/models", ~"models"),
			(~"GET", ~"/query-store", ~"query_store"),
			(~"GET", ~"/subject/{name}/{subject}", ~"subject"),
			(~"PUT", ~"/snmp-modeler", ~"modeler"),
		],
		views: ~[
			(~"map",  map_v),
			(~"shutdown",  bail_v),
			(~"models",  models_v),
			(~"query_store",  query_store_v),
			(~"subject",  subject_v),
			(~"modeler",  modeler_p),
		],
		sse: ~[(~"/query", query_s)],
		settings: ~[(~"debug",  ~"true")],		// TODO: make this a command-line option
		..server::initialize_config()
	};
	server::start(&config);
	
	info!("exiting gnos");						// won't normally land here
}
