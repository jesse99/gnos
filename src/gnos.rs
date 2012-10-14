use Path = path::Path;
use io::WriterUtil;
use std::json;
use std::map::*;
use server = rwebserve::rwebserve;
use handlers::*;
use rrdf::rrdf::*;
use task_runner::*;
use ConnConfig = rwebserve::connection::ConnConfig;
use Request = rwebserve::rwebserve::Request;
use Response = rwebserve::rwebserve::Response;
use ResponseHandler = rwebserve::rwebserve::ResponseHandler;

priv fn copy_scripts(root: &Path, user: ~str, host: ~str) -> option::Option<~str>
{
	let dir = core::os::make_absolute(root).pop();	// gnos/html => /gnos
	let dir = dir.push(~"scripts");						// /gnos => /gnos/scripts
	let files = utils::list_dir_path(&dir, ~[~".json", ~".py"]);
	
	utils::scp_files(files, user, host)
}

priv fn run_snmp(user: ~str, host: ~str, script: ~str) -> option::Option<~str>
{
	utils::run_remote_command(user, host, ~"python snmp-modeler.py -vv " + script)
}

priv fn snmp_exited(err: option::Option<~str>, state_chan: comm::Chan<model::Msg>)
{
	let mesg =
		if err.is_some()
		{
			~"snmp-modeler.py exited with stderr:\n" + err.get()
		}
		else
		{
			~"snmp-modeler.py exited with no stderr"
		};
	
	let lines = mesg.split_char('\n');
	for lines.each |line| {error!("%s", *line)};
	
	let alert = model::Alert {device: ~"gnos:map", id: ~"snmp-modeler.py exited", level: model::ErrorLevel, mesg: mesg, resolution: ~"Restart gnos."};	// TODO: probably should have a button somewhere to restart the script (would have to close the alert)
	comm::send(state_chan, model::UpdateMsg(~"alerts", |store, _err| {model::open_alert(store, &alert)}, ~""));
}

priv fn setup(options: &options::Options, state_chan: comm::Chan<model::Msg>) 
{
	let path = options.root.push(~"generated");				// html/generated
	if !os::path_is_dir(&path)
	{
		os::make_dir(&path, 7*8*8 + 7*8 + 7);
	}
	
	let root = copy options.root;
	let client2 = copy options.client;
	let cleanup = copy options.cleanup;
	
	let action: task_runner::JobFn = || copy_scripts(&root, env!("GNOS_USER"), client2);
	let cp = Job {action: action, policy: task_runner::ShutdownOnFailure};
	
	let client3 = copy options.client;
	let ccc = copy(options.script);
	let action: task_runner::JobFn = || run_snmp(env!("GNOS_USER"), client3, ccc);
	let run = Job {action: action, policy: task_runner::NotifyOnExit(|err| {snmp_exited(err, state_chan)})};
	
	task_runner::sequence(~[cp, run], cleanup);
}

priv fn get_shutdown(options: &options::Options) -> !
{
	info!("received shutdown request");
	for options.cleanup.each |f| {(*f)()};
	libc::exit(0)
}

priv fn update_globals(store: &Store, options: &options::Options) -> bool
{
	store.add(~"gnos:globals", ~[
		(~"gnos:admin", BoolValue(true)),		// TODO: get this from a setting
		(~"gnos:debug", BoolValue(true)),		// TODO: get this from command line
	]);
	
	let devices = vec::zip(vec::from_elem(options.devices.len(), ~"gnos:device"), do options.devices.map |n| {StringValue(copy n.managed_ip, ~"")});
	store.add(~"gnos:globals", devices);
	
	let names = model::get_standard_store_names();
	let stores = vec::zip(vec::from_elem(names.len(), ~"gnos:store"), do names.map |n| {StringValue(copy n, ~"")});
	store.add(~"gnos:globals", stores);
	
	true
}

fn static_view(options: &options::Options, config: &rwebserve::connection::ConnConfig, request: &Request, response: &Response) -> Response
{
	let response = rwebserve::configuration::static_view(config, request, response);
	
	// Generated images can be cached but, in general, they should expire just before we get new info.
	if request.path.starts_with("/generated/")
	{
		response.headers.insert(@~"Cache-Control", @fmt!("max-age=%?", options.poll_rate - 1));
	}
	response
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
	options::validate(&options);
	
	let state_chan = do task::spawn_listener |port, copy options| {model::manage_state(port, &options)};
	let samples_chan = do task::spawn_listener |port| {samples::manage_samples(port)};
	if !options.db
	{
		let client = copy options.client;
		let c1: task_runner::ExitFn = || {utils::run_remote_command(env!("GNOS_USER"), client, ~"pgrep -f snmp-modeler.py | xargs --no-run-if-empty kill -9");};
		options.cleanup = ~[c1];
		
		setup(&options, state_chan);
	}
	else
	{
		db::setup(state_chan, options.poll_rate);
	}
	
	let options1 = copy options;
	comm::send(state_chan, model::UpdateMsg(~"globals", |store, _err| {update_globals(store, &options1)}, ~""));
	
	// TODO: Shouldn't need all of these damned explicit types but rustc currently
	// has problems with type inference woth closures and borrowed pointers.
	let models_v: ResponseHandler = |_config: &ConnConfig, _request: &Request, response: &Response, copy options| {get_models::get_models(&options, response, state_chan)};
	let subject_v: ResponseHandler = |_config: &ConnConfig, request: &Request, response: &Response, copy options| {get_subject::get_subject(&options, request, response)};
	let home_v: ResponseHandler = |_config: &ConnConfig, _request: &Request, response: &Response, copy options| {get_home::get_home(&options, response)};
	let modeler_p: ResponseHandler = |_config: &ConnConfig, request: &Request, response: &Response, copy options| {put_snmp::put_snmp(&options, state_chan, samples_chan, request, response)};
	let query_store_v: ResponseHandler = |_config: &ConnConfig, request: &Request, response: &Response, copy options| {get_query_store::get_query_store(&options, request, response)};
	let bail_v: ResponseHandler = |_config: &ConnConfig, _request: &Request, _response: &Response, copy options| {get_shutdown(&options)};
	let interfaces_v: ResponseHandler = |_config: &ConnConfig, request: &Request, response: &Response| {get_interfaces::get_interfaces(request, response)};
	let static_v: ResponseHandler = |config: &ConnConfig, request: &Request, response: &Response, copy options| {static_view(&options, config, request, response)};
	
	let query_s: server::OpenSse = |_config: &ConnConfig, request: &Request, push| {sse_query::sse_query(state_chan, request, push)};
	let samples_s: server::OpenSse = |_config: &ConnConfig, request: &Request, push| {sse_samples::sse_query(samples_chan, request, push)};
	
	let config = server::Config
	{
		// We need to bind to the server addresses so that we receive modeler PUTs.
		// We bind to localhost to ensure that we can hit the web server using a local
		// browser.
		hosts: if options.db {~[~"localhost"]} else if options.admin {~[copy options.server, ~"localhost"]} else {~[copy options.server]},
		port: options.port,
		server_info: ~"gnos " + options::get_version(),
		resources_root: options.root,
		routes: ~[
			(~"GET", ~"/", ~"home"),
			(~"GET", ~"/interfaces/{managed_ip}/{direction}", ~"interfaces"),
			(~"GET", ~"/shutdown", ~"shutdown"),		// TODO: enable this via debug cfg (or maybe via a command line option)
			(~"GET", ~"/models", ~"models"),
			(~"GET", ~"/query-store", ~"query_store"),
			(~"GET", ~"/subject/{name}/{subject}", ~"subject"),
			(~"PUT", ~"/snmp-modeler", ~"modeler"),
		],
		views: ~[
			(~"home",  home_v),
			(~"interfaces",  interfaces_v),
			(~"shutdown",  bail_v),
			(~"models",  models_v),
			(~"query_store",  query_store_v),
			(~"subject",  subject_v),
			(~"modeler",  modeler_p),
		],
		static_handler: static_v,
		sse: ~[(~"/query", query_s), (~"/samples", samples_s)],
		settings: ~[(~"debug",  ~"true")],		// TODO: make this a command-line option
		..server::initialize_config()
	};
	
	// There is a bit of a chicken and egg problem here: ideally we'd start up the web page after the server
	// starts but before it exits. For a while I was starting up the browser in the make file before launching
	// the server. This worked fine for a while but Chrome started timing out as the server began to do
	// more work on startup. Hopefully this will work better.
	if options.browse.is_not_empty()
	{
		let url = copy options.browse;
		do task::spawn {core::run::program_output("git", ~[~"web--browse", copy url]);}
	}
	
	server::start(&config);
	info!("exiting gnos");						// won't normally land here
}
